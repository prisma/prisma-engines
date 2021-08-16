use super::{common::*, SqlRenderer};
use crate::{
    flavour::SqliteFlavour,
    pair::Pair,
    sql_migration::{AlterEnum, AlterTable, RedefineTable, TableChange},
};
use indoc::formatdoc;
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use sql_ddl::sqlite as ddl;
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

        let index_create = format!(
            "CREATE {index_type}INDEX {index_name} ON {table_reference}({columns})",
            index_type = index_type,
            index_name = index_name,
            table_reference = table_reference,
            columns = columns.join(", ")
        );

        if index.name().starts_with("sqlite_") {
            formatdoc!(
                "Pragma writable_schema=1;
                 {};
                 Pragma writable_schema=0",
                index_create
            )
        } else {
            index_create
        }
    }

    fn render_add_foreign_key(&self, _foreign_key: &ForeignKeyWalker<'_>) -> String {
        unreachable!("AddForeignKey on SQLite")
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: &Pair<&SqlSchema>) -> Vec<String> {
        let AlterTable {
            changes,
            table_ids: table_index,
        } = alter_table;

        let tables = schemas.tables(table_index);

        let mut statements = Vec::new();

        // See https://www.sqlite.org/lang_altertable.html for the reference on
        // what is possible on SQLite.

        for change in changes {
            match change {
                TableChange::AddColumn { column_id } => {
                    let column = tables.next().column_at(*column_id);
                    let col_sql = render_column(&column);

                    statements.push(format!(
                        "ALTER TABLE {table_name} ADD COLUMN {column_definition}",
                        table_name = self.quote(tables.previous().name()),
                        column_definition = col_sql,
                    ));
                }
                TableChange::AddPrimaryKey { .. } => unreachable!("AddPrimaryKey on SQLite"),
                TableChange::AlterColumn(_) => unreachable!("AlterColumn on SQLite"),
                TableChange::DropAndRecreateColumn { .. } => unreachable!("DropAndRecreateColumn on SQLite"),
                TableChange::DropColumn { .. } => unreachable!("DropColumn on SQLite"),
                TableChange::DropPrimaryKey { .. } => unreachable!("DropPrimaryKey on SQLite"),
                TableChange::RenamePrimaryKey { .. } => unreachable!("AddPrimaryKey on SQLite"),
            };
        }

        statements
    }

    fn render_create_enum(&self, _: &EnumWalker<'_>) -> Vec<String> {
        unreachable!("Unreachable render_create_enum() on SQLite. SQLite does not have enums.")
    }

    fn render_create_table_as(&self, table: &TableWalker<'_>, table_name: &str) -> String {
        let mut create_table = sql_ddl::sqlite::CreateTable {
            table_name: table_name.into(),
            columns: table.columns().map(|col| render_column(&col)).collect(),
            primary_key: None,
            foreign_keys: table
                .foreign_keys()
                .map(move |fk| sql_ddl::sqlite::ForeignKey {
                    constrains: fk.constrained_column_names().iter().map(|name| name.into()).collect(),
                    references: (
                        fk.referenced_table().name().into(),
                        fk.referenced_column_names().iter().map(|name| name.into()).collect(),
                    ),
                    constraint_name: fk.constraint_name().map(From::from),
                    on_delete: Some(match fk.on_delete_action() {
                        ForeignKeyAction::NoAction => sql_ddl::sqlite::ForeignKeyAction::NoAction,
                        ForeignKeyAction::Restrict => sql_ddl::sqlite::ForeignKeyAction::Restrict,
                        ForeignKeyAction::Cascade => sql_ddl::sqlite::ForeignKeyAction::Cascade,
                        ForeignKeyAction::SetNull => sql_ddl::sqlite::ForeignKeyAction::SetNull,
                        ForeignKeyAction::SetDefault => sql_ddl::sqlite::ForeignKeyAction::SetDefault,
                    }),
                    on_update: Some(match fk.on_update_action() {
                        ForeignKeyAction::NoAction => sql_ddl::sqlite::ForeignKeyAction::NoAction,
                        ForeignKeyAction::Restrict => sql_ddl::sqlite::ForeignKeyAction::Restrict,
                        ForeignKeyAction::Cascade => sql_ddl::sqlite::ForeignKeyAction::Cascade,
                        ForeignKeyAction::SetNull => sql_ddl::sqlite::ForeignKeyAction::SetNull,
                        ForeignKeyAction::SetDefault => sql_ddl::sqlite::ForeignKeyAction::SetDefault,
                    }),
                })
                .collect(),
        };

        if !table.columns().any(|col| col.is_single_primary_key()) {
            create_table.primary_key = table
                .primary_key_column_names()
                .map(|slice| slice.iter().map(|name| name.into()).collect());
        }

        create_table.to_string()
    }

    fn render_drop_enum(&self, _: &EnumWalker<'_>) -> Vec<String> {
        unreachable!("Unreachable render_drop_enum() on SQLite. SQLite does not have enums.")
    }

    fn render_drop_foreign_key(&self, _foreign_key: &ForeignKeyWalker<'_>) -> String {
        unreachable!("render_drop_foreign_key on SQLite")
    }

    fn render_drop_index(&self, index: &IndexWalker<'_>) -> String {
        format!("DROP INDEX {}", self.quote(index.name()))
    }

    fn render_drop_and_recreate_index(&self, _indexes: Pair<&IndexWalker<'_>>) -> Vec<String> {
        vec![
            self.render_drop_index(*_indexes.previous()),
            self.render_create_index(*_indexes.next()),
        ]
    }

    fn render_drop_table(&self, table_name: &str) -> Vec<String> {
        // Turning off the pragma is safe, because schema validation would forbid foreign keys
        // to a non-existent model. There appears to be no other way to deal with cyclic
        // dependencies in the dropping order of tables in the presence of foreign key
        // constraints on SQLite.
        vec![
            "PRAGMA foreign_keys=off".to_string(),
            format!("DROP TABLE {}", self.quote(table_name)),
            "PRAGMA foreign_keys=on".to_string(),
        ]
    }

    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: &Pair<&SqlSchema>) -> Vec<String> {
        // Based on 'Making Other Kinds Of Table Schema Changes' from https://www.sqlite.org/lang_altertable.html
        let mut result = vec!["PRAGMA foreign_keys=OFF".to_string()];

        for redefine_table in tables {
            let tables = schemas.tables(&redefine_table.table_ids);
            let temporary_table_name = format!("new_{}", &tables.next().name());

            result.push(self.render_create_table_as(tables.next(), &temporary_table_name));

            copy_current_table_into_new_table(&mut result, redefine_table, &tables, &temporary_table_name);

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

    fn render_drop_view(&self, view: &ViewWalker<'_>) -> String {
        format!(r#"DROP VIEW "{}""#, view.name())
    }

    fn render_drop_user_defined_type(&self, _: &UserDefinedTypeWalker<'_>) -> String {
        unreachable!("render_drop_user_defined_type on SQLite")
    }
}

fn render_column_type(t: &ColumnType) -> &str {
    match &t.family {
        ColumnTypeFamily::Boolean => "BOOLEAN",
        ColumnTypeFamily::DateTime => "DATETIME",
        ColumnTypeFamily::Float => "REAL",
        ColumnTypeFamily::Decimal => "DECIMAL",
        ColumnTypeFamily::Int => "INTEGER",
        ColumnTypeFamily::BigInt => "BIGINT",
        ColumnTypeFamily::String => "TEXT",
        ColumnTypeFamily::Binary => "BLOB",
        ColumnTypeFamily::Json => unreachable!("ColumnTypeFamily::Json on SQLite"),
        ColumnTypeFamily::Enum(_) => unreachable!("ColumnTypeFamily::Enum on SQLite"),
        ColumnTypeFamily::Uuid => unimplemented!("ColumnTypeFamily::Uuid on SQLite"),
        ColumnTypeFamily::Unsupported(x) => x.as_ref(),
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
) {
    if redefine_table.column_pairs.is_empty() {
        return;
    }

    let destination_columns = redefine_table
        .column_pairs
        .iter()
        .map(|(column_ides, _, _)| tables.next().column_at(*column_ides.next()).name());

    let source_columns = redefine_table.column_pairs.iter().map(|(column_ides, changes, _)| {
        let columns = tables.columns(column_ides);

        let col_became_required_with_a_default =
            changes.arity_changed() && columns.next().arity().is_required() && columns.next().default().is_some();

        if col_became_required_with_a_default {
            format!(
                "coalesce({column_name}, {default_value}) AS {column_name}",
                column_name = Quoted::sqlite_ident(columns.previous().name()),
                default_value = render_default(
                    columns
                        .next()
                        .default()
                        .expect("default on required column with default"),
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

fn render_column<'a>(column: &ColumnWalker<'a>) -> ddl::Column<'a> {
    sql_ddl::sqlite::Column {
        autoincrement: column.is_single_primary_key() && column.column_type_family().is_int(),
        default: column
            .default()
            .filter(|default| !matches!(default.kind(), DefaultKind::Sequence(_)))
            .map(|default| render_default(default)),
        name: column.name().into(),
        not_null: !column.arity().is_nullable(),
        primary_key: column.is_single_primary_key(),
        r#type: render_column_type(column.column_type()).into(),
    }
}

fn render_default(default: &DefaultValue) -> Cow<'_, str> {
    match default.kind() {
        DefaultKind::DbGenerated(val) => val.as_str().into(),
        DefaultKind::Value(PrismaValue::String(val)) | DefaultKind::Value(PrismaValue::Enum(val)) => {
            Quoted::sqlite_string(escape_quotes(val)).to_string().into()
        }
        DefaultKind::Value(PrismaValue::Bytes(b)) => Quoted::sqlite_string(format_hex(b)).to_string().into(),
        DefaultKind::Now => "CURRENT_TIMESTAMP".into(),
        DefaultKind::Value(PrismaValue::DateTime(val)) => Quoted::sqlite_string(val).to_string().into(),
        DefaultKind::Value(val) => format!("{}", val).into(),
        DefaultKind::Sequence(_) => "".into(),
    }
}
