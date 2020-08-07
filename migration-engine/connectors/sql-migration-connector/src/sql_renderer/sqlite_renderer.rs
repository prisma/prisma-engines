use super::{common::*, RenderedAlterColumn, SqlRenderer};
use crate::{
    database_info::DatabaseInfo,
    flavour::{SqlFlavour, SqliteFlavour},
    sql_database_step_applier::render_create_index,
    sql_schema_differ::{ColumnDiffer, SqlSchemaDiffer, TableDiffer},
    sql_schema_helpers::*,
};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use sql_schema_describer::*;
use std::borrow::Cow;

impl SqlRenderer for SqliteFlavour {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::Double(name)
    }

    fn render_column(&self, _schema_name: &str, column: ColumnRef<'_>, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(column.column_type());
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .filter(|default| !matches!(default, DefaultValue::DBGENERATED(_) | DefaultValue::SEQUENCE(_)))
            .map(|default| format!(" DEFAULT {}", self.render_default(default, &column.column.tpe.family)))
            .unwrap_or_else(String::new);
        let auto_increment_str = if column.is_autoincrement() && column.is_single_primary_key() {
            " PRIMARY KEY AUTOINCREMENT"
        } else {
            ""
        };

        format!(
            "{column_name} {tpe_str} {nullability_str}{default_str}{auto_increment}",
            column_name = column_name,
            tpe_str = tpe_str,
            nullability_str = nullability_str,
            default_str = default_str,
            auto_increment = auto_increment_str
        )
    }

    fn render_references(&self, _schema_name: &str, foreign_key: &ForeignKey) -> String {
        let referenced_fields = foreign_key
            .referenced_columns
            .iter()
            .map(Quoted::sqlite_ident)
            .join(",");

        format!(
            "REFERENCES {referenced_table}({referenced_fields}) {on_delete_action} ON UPDATE CASCADE",
            referenced_table = self.quote(&foreign_key.referenced_table),
            referenced_fields = referenced_fields,
            on_delete_action = render_on_delete(&foreign_key.on_delete_action)
        )
    }

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str> {
        match (default, family) {
            (DefaultValue::DBGENERATED(val), _) => val.as_str().into(),
            (DefaultValue::VALUE(PrismaValue::String(val)), ColumnTypeFamily::String)
            | (DefaultValue::VALUE(PrismaValue::Enum(val)), ColumnTypeFamily::Enum(_)) => {
                format!("'{}'", escape_quotes(&val)).into()
            }
            (DefaultValue::NOW, ColumnTypeFamily::DateTime) => "CURRENT_TIMESTAMP".into(),
            (DefaultValue::NOW, _) => unreachable!("NOW default on non-datetime column"),
            (DefaultValue::VALUE(val), ColumnTypeFamily::DateTime) => format!("'{}'", val).into(),
            (DefaultValue::VALUE(val), _) => format!("{}", val).into(),
            (DefaultValue::SEQUENCE(_), _) => "".into(),
        }
    }

    fn render_alter_column(&self, _differ: &ColumnDiffer<'_>) -> Option<RenderedAlterColumn> {
        None
    }

    fn render_create_enum(&self, _create_enum: &crate::CreateEnum) -> Vec<String> {
        Vec::new()
    }

    fn render_drop_enum(&self, _drop_enum: &crate::DropEnum) -> Vec<String> {
        Vec::new()
    }

    fn render_redefine_tables(
        &self,
        tables: &[String],
        differ: SqlSchemaDiffer<'_>,
        database_info: &DatabaseInfo,
    ) -> Vec<String> {
        // Based on 'Making Other Kinds Of Table Schema Changes' from https://www.sqlite.org/lang_altertable.html
        let mut result: Vec<String> = Vec::new();
        let schema_name = database_info.connection_info().schema_name();
        let sql_family = database_info.sql_family();

        result.push("PRAGMA foreign_keys=OFF".to_string());

        for table in tables {
            let differ = TableDiffer {
                database_info,
                flavour: self,
                previous: differ.previous.table_ref(table).expect(""),
                next: differ.next.table_ref(table).expect(""),
            };

            let name_of_temporary_table = format!("new_{}", &differ.next.name());
            let mut temporary_table = differ.next.table.clone();
            temporary_table.name = name_of_temporary_table.clone();

            // This is a hack, just to be able to render the CREATE TABLE.
            let temporary_table = TableRef {
                schema: differ.next.schema,
                table: &temporary_table,
            };

            // TODO start transaction now. Unclear if we really want to do that.
            result.push(
                self.render_create_table(&temporary_table, schema_name, sql_family)
                    .expect("render_create_table"),
            );

            copy_current_table_into_new_table(&mut result, &differ, temporary_table.name(), schema_name, self).unwrap();

            result.push(format!("DROP TABLE \"{}\".\"{}\"", schema_name, differ.next.name()));

            result.push(format!(
                "ALTER TABLE \"{schema_name}\".\"{old_name}\" RENAME TO \"{new_name}\"",
                schema_name = schema_name,
                old_name = temporary_table.name(),
                new_name = differ.next.name()
            ));

            // Recreate the indices
            result.extend(
                differ
                    .next
                    .table
                    .indices
                    .iter()
                    .map(|index| render_create_index(self, schema_name, differ.next.name(), index, sql_family)),
            );
        }

        result.push(format!(
            "PRAGMA {}.foreign_key_check",
            Quoted::sqlite_ident(schema_name)
        ));

        result.push("PRAGMA foreign_keys=ON".to_string());

        result
    }
}

fn render_column_type(t: &ColumnType) -> String {
    match &t.family {
        ColumnTypeFamily::Boolean => "BOOLEAN".to_string(),
        ColumnTypeFamily::DateTime => "DATETIME".to_string(),
        ColumnTypeFamily::Float => "REAL".to_string(),
        ColumnTypeFamily::Int => "INTEGER".to_string(),
        ColumnTypeFamily::String => "TEXT".to_string(),
        x => unimplemented!("{:?} not handled yet", x),
    }
}

fn escape_quotes(s: &str) -> Cow<'_, str> {
    static STRING_LITERAL_CHARACTER_TO_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'"#).unwrap());

    STRING_LITERAL_CHARACTER_TO_ESCAPE_RE.replace_all(s, "'$0")
}

/// Copy the existing data into the new table.
///
/// The process is complicated by the migrations that add make an optional column required with a
/// default value. In this case, we need to treat them differently and `coalesce`ing them with the
/// default value, since SQLite does not have the `DEFAULT` keyword.
fn copy_current_table_into_new_table(
    steps: &mut Vec<String>,
    differ: &TableDiffer<'_>,
    temporary_table: &str,
    schema_name: &str,
    flavour: &dyn SqlFlavour,
) -> std::fmt::Result {
    use std::fmt::Write as _;

    let columns_that_became_required_with_a_default: Vec<ColumnDiffer<'_>> = differ
        .column_pairs()
        .filter(|columns| {
            columns.all_changes().arity_changed()
                && columns.next.column.tpe.arity.is_required()
                && columns.next.column.default.is_some()
        })
        .collect();

    let intersection_columns: Vec<&str> = differ
        .column_pairs()
        .filter(|columns| {
            !columns_that_became_required_with_a_default
                .iter()
                .any(|excluded| excluded.name() == columns.name())
        })
        .map(|columns| columns.name())
        .collect();

    let mut query = String::with_capacity(40);

    write!(
        query,
        "INSERT INTO {}.{} (",
        Quoted::sqlite_ident(schema_name),
        Quoted::sqlite_ident(temporary_table)
    )?;

    let mut destination_columns = intersection_columns
        .iter()
        .copied()
        .chain(
            columns_that_became_required_with_a_default
                .iter()
                .map(|columns| columns.name()),
        )
        .peekable();

    while let Some(destination_column) = destination_columns.next() {
        write!(query, "{}", Quoted::sqlite_ident(destination_column))?;

        if destination_columns.peek().is_some() {
            write!(query, ", ")?;
        }
    }

    write!(query, r#") SELECT "#)?;

    let mut source_columns = intersection_columns
        .iter()
        .map(|s| format!("{}", Quoted::sqlite_ident(s)))
        .chain(columns_that_became_required_with_a_default.iter().map(|columns| {
            format!(
                "coalesce({column_name}, {default_value}) AS {column_name}",
                column_name = Quoted::sqlite_ident(columns.name()),
                default_value = flavour.render_default(
                    columns
                        .next
                        .column
                        .default
                        .as_ref()
                        .expect("default on required column with default"),
                    &columns.next.column.tpe.family
                )
            )
        }))
        .peekable();

    while let Some(source_column) = source_columns.next() {
        write!(query, "{}", source_column)?;

        if source_columns.peek().is_some() {
            write!(query, ", ")?;
        }
    }

    write!(
        query,
        " FROM {}.{}",
        Quoted::sqlite_ident(schema_name),
        Quoted::sqlite_ident(&differ.next.name())
    )?;

    steps.push(query);

    Ok(())
}
