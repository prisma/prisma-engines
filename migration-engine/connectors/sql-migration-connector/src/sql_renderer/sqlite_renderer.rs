use super::{common::*, SqlRenderer};
use crate::{
    flavour::SqliteFlavour,
    pair::Pair,
    sql_migration::{AddColumn, AlterEnum, AlterTable, DropForeignKey, DropIndex, RedefineTable, TableChange},
};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use sql_schema_describer::{walkers::*, *};
use std::borrow::Cow;

impl SqlRenderer for SqliteFlavour {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::Double(name)
    }

    fn render_alter_enum(&self, _alter_enum: &AlterEnum, _schemas: &Pair<&SqlSchema>) -> Vec<String> {
        unreachable!("render_alter_enum on sqlite")
    }

    fn render_create_index(&self, index: &IndexWalker<'_>) -> String {
        let index_type = match index.index_type() {
            IndexType::Unique => "UNIQUE ",
            IndexType::Normal => "",
        };
        let index_name = self.quote(index.name());
        let table_reference = self.quote(index.table().name());
        let columns = index.columns().map(|c| self.quote(c.name()));

        format!(
            "CREATE {index_type}INDEX {index_name} ON {table_reference}({columns})",
            index_type = index_type,
            index_name = index_name,
            table_reference = table_reference,
            columns = columns.join(", ")
        )
    }

    fn render_column(&self, column: &ColumnWalker<'_>) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(column.column_type());
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .filter(|default| !matches!(default, DefaultValue::DBGENERATED(_) | DefaultValue::SEQUENCE(_)))
            .map(|default| format!(" DEFAULT {}", self.render_default(default, column.column_type_family())))
            .unwrap_or_else(String::new);
        let auto_increment_str = if column.is_autoincrement() && column.is_single_primary_key() {
            " PRIMARY KEY AUTOINCREMENT"
        } else {
            ""
        };

        format!(
            "{indentation}{column_name} {tpe_str}{nullability_str}{default_str}{auto_increment}",
            indentation = SQL_INDENTATION,
            column_name = column_name,
            tpe_str = tpe_str,
            nullability_str = nullability_str,
            default_str = default_str,
            auto_increment = auto_increment_str
        )
    }

    fn render_references(&self, foreign_key: &ForeignKeyWalker<'_>) -> String {
        let referenced_fields = foreign_key
            .referenced_column_names()
            .iter()
            .map(Quoted::sqlite_ident)
            .join(",");

        format!(
            "REFERENCES {referenced_table}({referenced_fields}) {on_delete_action} ON UPDATE CASCADE",
            referenced_table = self.quote(foreign_key.referenced_table().name()),
            referenced_fields = referenced_fields,
            on_delete_action = render_on_delete(foreign_key.on_delete_action())
        )
    }

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str> {
        match (default, family) {
            (DefaultValue::DBGENERATED(val), _) => val.as_str().into(),
            (DefaultValue::VALUE(PrismaValue::String(val)), ColumnTypeFamily::String)
            | (DefaultValue::VALUE(PrismaValue::Enum(val)), ColumnTypeFamily::Enum(_)) => {
                format!("'{}'", escape_quotes(&val)).into()
            }
            (DefaultValue::VALUE(PrismaValue::Bytes(b)), ColumnTypeFamily::Binary) => {
                format!("'{}'", format_hex(b)).into()
            }
            (DefaultValue::NOW, ColumnTypeFamily::DateTime) => "CURRENT_TIMESTAMP".into(),
            (DefaultValue::NOW, _) => unreachable!("NOW default on non-datetime column"),
            (DefaultValue::VALUE(val), ColumnTypeFamily::DateTime) => format!("'{}'", val).into(),
            (DefaultValue::VALUE(val), _) => format!("{}", val).into(),
            (DefaultValue::SEQUENCE(_), _) => "".into(),
        }
    }

    fn render_add_foreign_key(&self, _foreign_key: &ForeignKeyWalker<'_>) -> String {
        unreachable!("AddForeignKey on SQLite")
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: &Pair<&SqlSchema>) -> Vec<String> {
        let AlterTable { changes, table_index } = alter_table;

        let tables = schemas.tables(table_index);

        let mut statements = Vec::new();

        // See https://www.sqlite.org/lang_altertable.html for the reference on
        // what is possible on SQLite.

        for change in changes {
            match change {
                TableChange::AddColumn(AddColumn { column_index }) => {
                    let column = tables.next().column_at(*column_index);
                    let col_sql = self.render_column(&column);

                    statements.push(format!(
                        "ALTER TABLE {table_name} ADD COLUMN {column_definition}",
                        table_name = self.quote(tables.previous().name()),
                        column_definition = col_sql,
                    ));
                }
                TableChange::AddPrimaryKey { .. } => unreachable!("AddPrimaryKey on SQLite"),
                TableChange::AlterColumn(_) => unreachable!("AlterColumn on SQLite"),
                TableChange::DropAndRecreateColumn { .. } => unreachable!("DropAndRecreateColumn on SQLite"),
                TableChange::DropColumn(_) => unreachable!("DropColumn on SQLite"),
                TableChange::DropPrimaryKey { .. } => unreachable!("DropPrimaryKey on SQLite"),
            };
        }

        statements
    }

    fn render_create_enum(&self, _: &EnumWalker<'_>) -> Vec<String> {
        Vec::new()
    }

    fn render_create_table_as(&self, table: &TableWalker<'_>, table_name: &str) -> String {
        use std::fmt::Write;

        let columns: String = table.columns().map(|column| self.render_column(&column)).join(",\n");

        let primary_key_is_already_set = columns.contains("PRIMARY KEY");
        let primary_columns = table.primary_key_column_names().unwrap_or(&[]);

        let primary_key = if !primary_columns.is_empty() && !primary_key_is_already_set {
            let column_names = primary_columns.iter().map(Quoted::sqlite_ident).join(",");
            format!(
                ",\n{indentation}PRIMARY KEY ({column_names})",
                indentation = SQL_INDENTATION,
                column_names = column_names
            )
        } else {
            String::new()
        };

        let foreign_keys = if !table.foreign_keys().next().is_none() {
            let mut fks = table.foreign_keys().peekable();
            let mut rendered_fks = String::new();

            while let Some(fk) = fks.next() {
                write!(
                    rendered_fks,
                    "{indentation}{constraint_clause}FOREIGN KEY ({constrained_columns}) {references}{comma}",
                    constraint_clause = fk
                        .constraint_name()
                        .map(|name| format!("CONSTRAINT {} ", name))
                        .unwrap_or_default(),
                    indentation = SQL_INDENTATION,
                    constrained_columns = fk
                        .constrained_column_names()
                        .iter()
                        .map(|col| format!(r#""{}""#, col))
                        .join(","),
                    references = self.render_references(&fk),
                    comma = if fks.peek().is_some() { ",\n" } else { "" },
                )
                .expect("Error formatting to string buffer.");
            }

            format!(",\n\n{fks}", fks = rendered_fks)
        } else {
            String::new()
        };

        format!(
            "CREATE TABLE {table_name} (\n{columns}{foreign_keys}{primary_key}\n)",
            table_name = self.quote(table_name),
            columns = columns,
            foreign_keys = foreign_keys,
            primary_key = primary_key,
        )
    }

    fn render_drop_enum(&self, _: &EnumWalker<'_>) -> Vec<String> {
        Vec::new()
    }

    fn render_drop_foreign_key(&self, _drop_foreign_key: &DropForeignKey) -> String {
        unreachable!("render_drop_foreign_key on SQLite")
    }

    fn render_drop_index(&self, drop_index: &DropIndex) -> String {
        format!("DROP INDEX {}", self.quote(&drop_index.name))
    }

    fn render_drop_table(&self, table_name: &str) -> Vec<String> {
        // Turning off the pragma is safe, because schema validation would forbid foreign keys
        // to a non-existent model. There appears to be no other way to deal with cyclic
        // dependencies in the dropping order of tables in the presence of foreign key
        // constraints on SQLite.
        vec![
            "PRAGMA foreign_keys=off".to_string(),
            format!("DROP TABLE {}", self.quote(&table_name)),
            "PRAGMA foreign_keys=on".to_string(),
        ]
    }

    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: &Pair<&SqlSchema>) -> Vec<String> {
        // Based on 'Making Other Kinds Of Table Schema Changes' from https://www.sqlite.org/lang_altertable.html
        let mut result: Vec<String> = Vec::new();

        result.push("PRAGMA foreign_keys=OFF".to_string());

        for redefine_table in tables {
            let tables = schemas.tables(&redefine_table.table_index);
            let temporary_table_name = format!("new_{}", &tables.next().name());

            result.push(self.render_create_table_as(tables.next(), &temporary_table_name));

            copy_current_table_into_new_table(&mut result, redefine_table, &tables, &temporary_table_name, self);

            result.push(format!(r#"DROP TABLE "{}""#, tables.previous().name()));

            result.push(format!(
                r#"ALTER TABLE "{old_name}" RENAME TO "{new_name}""#,
                old_name = temporary_table_name,
                new_name = tables.next().name(),
            ));

            for index in tables.next().indexes() {
                result.push(self.render_create_index(&index));
            }
        }

        result.push("PRAGMA foreign_key_check".to_string());
        result.push("PRAGMA foreign_keys=ON".to_string());

        result
    }

    fn render_rename_table(&self, name: &str, new_name: &str) -> String {
        format!(r#"ALTER TABLE "{}" RENAME TO "{}""#, name, new_name)
    }
}

fn render_column_type(t: &ColumnType) -> &'static str {
    match &t.family {
        ColumnTypeFamily::Boolean => "BOOLEAN",
        ColumnTypeFamily::DateTime => "DATETIME",
        ColumnTypeFamily::Float => "REAL",
        ColumnTypeFamily::Decimal => "REAL",
        ColumnTypeFamily::Int => "INTEGER",
        ColumnTypeFamily::BigInt => "INTEGER",
        ColumnTypeFamily::String => "TEXT",
        ColumnTypeFamily::Binary => "BLOB",
        ColumnTypeFamily::Json => unreachable!("ColumnTypeFamily::Json on SQLite"),
        ColumnTypeFamily::Enum(_) => unreachable!("ColumnTypeFamily::Enum on SQLite"),
        ColumnTypeFamily::Uuid => unimplemented!("ColumnTypeFamily::Uuid on SQLite"),
        ColumnTypeFamily::Unsupported(x) => unimplemented!("{} not handled yet", x),
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
    redefine_table: &RedefineTable,
    tables: &Pair<TableWalker<'_>>,
    temporary_table_name: &str,
    flavour: &SqliteFlavour,
) {
    let destination_columns = redefine_table
        .column_pairs
        .iter()
        .map(|(column_indexes, _, _)| tables.next().column_at(*column_indexes.next()).name());

    let source_columns = redefine_table.column_pairs.iter().map(|(column_indexes, changes, _)| {
        let columns = tables.columns(column_indexes);

        let col_became_required_with_a_default =
            changes.arity_changed() && columns.next().arity().is_required() && columns.next().default().is_some();

        if col_became_required_with_a_default {
            format!(
                "coalesce({column_name}, {default_value}) AS {column_name}",
                column_name = Quoted::sqlite_ident(columns.previous().name()),
                default_value = flavour.render_default(
                    columns
                        .next()
                        .default()
                        .expect("default on required column with default"),
                    &columns.next().column_type_family()
                )
            )
        } else {
            Quoted::sqlite_ident(columns.previous().name()).to_string()
        }
    });

    let query = format!(
        r#"INSERT INTO "{temporary_table_name}" ({destination_columns}) SELECT {source_columns} FROM "{previous_table_name}""#,
        temporary_table_name = temporary_table_name,
        destination_columns = destination_columns.map(Quoted::sqlite_ident).join(", "),
        source_columns = source_columns.join(", "),
        previous_table_name = tables.previous().name(),
    );

    steps.push(query)
}
