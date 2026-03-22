use crate::sql_renderer::{IteratorJoin, Quoted, QuotedWithPrefix, SqlRenderer};
use crate::{
    migration_pair::MigrationPair,
    sql_migration::{AlterEnum, AlterTable, RedefineTable, TableChange},
};
use sql_schema_describer::{walkers::*, *};
use std::borrow::Cow;

#[derive(Debug)]
pub struct SurrealDbRenderer;

impl SqlRenderer for SurrealDbRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::Backticks(name)
    }

    fn render_alter_enum(&self, _alter_enum: &AlterEnum, _schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        unreachable!("render_alter_enum on SurrealDB")
    }

    fn render_create_index(&self, index: IndexWalker<'_>) -> String {
        let index_type = if index.is_unique() { "UNIQUE " } else { "" };
        let index_name = self.quote(index.name());
        let table_reference = self.quote(index.table().name());

        let columns = index.columns().map(|c| {
            let mut rendered = format!("{}", self.quote(c.as_column().name()));
            if let Some(sort_order) = c.sort_order() {
                rendered.push(' ');
                rendered.push_str(sort_order.as_ref());
            }
            rendered
        });

        format!(
            "DEFINE INDEX {index_name} ON TABLE {table_reference} FIELDS {columns} {index_type}",
            index_name = index_name,
            table_reference = table_reference,
            columns = columns.join(", "),
            index_type = index_type.trim(),
        )
        .trim()
        .to_owned()
    }

    fn render_add_foreign_key(&self, _foreign_key: ForeignKeyWalker<'_>) -> String {
        // SurrealDB does not support foreign key constraints
        String::new()
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        let AlterTable { changes, table_ids } = alter_table;
        let tables = schemas.walk(*table_ids);
        let mut statements = Vec::new();

        for change in changes {
            match change {
                TableChange::AddColumn {
                    column_id,
                    has_virtual_default: _,
                } => {
                    let column = schemas.next.walk(*column_id);
                    let col_type = render_column_type(column.column_type());
                    let nullable = if column.arity().is_nullable() {
                        " | NONE"
                    } else {
                        ""
                    };

                    let mut field_def = format!(
                        "DEFINE FIELD {field_name} ON TABLE {table_name} TYPE {field_type}{nullable}",
                        field_name = self.quote(column.name()),
                        table_name = self.quote(tables.previous.name()),
                        field_type = col_type,
                        nullable = nullable,
                    );

                    if let Some(default) = column.default() {
                        if !matches!(default.kind(), DefaultKind::Sequence(_) | DefaultKind::DbGenerated(None)) {
                            field_def.push_str(&format!(" DEFAULT {}", render_default(default.inner())));
                        }
                    }

                    statements.push(field_def);
                }
                TableChange::DropColumn { column_id } => {
                    let column = schemas.previous.walk(*column_id);
                    statements.push(format!(
                        "REMOVE FIELD {field_name} ON TABLE {table_name}",
                        field_name = self.quote(column.name()),
                        table_name = self.quote(tables.previous.name()),
                    ));
                }
                TableChange::AlterColumn(_alter_column) => {
                    // SurrealDB handles field redefinition by re-issuing DEFINE FIELD
                }
                TableChange::DropAndRecreateColumn { column_id, .. } => {
                    let previous_column = schemas.previous.walk(column_id.previous);
                    let next_column = schemas.next.walk(column_id.next);
                    let col_type = render_column_type(next_column.column_type());
                    let nullable = if next_column.arity().is_nullable() {
                        " | NONE"
                    } else {
                        ""
                    };

                    statements.push(format!(
                        "REMOVE FIELD {field_name} ON TABLE {table_name}",
                        field_name = self.quote(previous_column.name()),
                        table_name = self.quote(tables.previous.name()),
                    ));
                    statements.push(format!(
                        "DEFINE FIELD {field_name} ON TABLE {table_name} TYPE {field_type}{nullable}",
                        field_name = self.quote(next_column.name()),
                        table_name = self.quote(tables.previous.name()),
                        field_type = col_type,
                        nullable = nullable,
                    ));
                }
                TableChange::AddPrimaryKey => {}
                TableChange::DropPrimaryKey => {}
                TableChange::RenamePrimaryKey => {}
            };
        }

        statements
    }

    fn render_create_enum(&self, _: EnumWalker<'_>) -> Vec<String> {
        unreachable!("SurrealDB does not have enums")
    }

    fn render_create_table(&self, table: TableWalker<'_>) -> String {
        self.render_create_table_as(table, QuotedWithPrefix(None, self.quote(table.name())))
    }

    fn render_create_table_as(&self, table: TableWalker<'_>, table_name: QuotedWithPrefix<&str>) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push(format!("DEFINE TABLE {table_name} SCHEMAFULL"));

        for column in table.columns() {
            let col_type = render_column_type(column.column_type());
            let nullable = if column.arity().is_nullable() {
                " | NONE"
            } else {
                ""
            };

            let mut field_def = format!(
                "DEFINE FIELD {field_name} ON TABLE {table_name} TYPE {field_type}{nullable}",
                field_name = Quoted::Backticks(column.name()),
                table_name = table_name,
                field_type = col_type,
                nullable = nullable,
            );

            if let Some(default) = column.default() {
                if !matches!(default.kind(), DefaultKind::Sequence(_) | DefaultKind::DbGenerated(None) | DefaultKind::UniqueRowid) {
                    field_def.push_str(&format!(" DEFAULT {}", render_default(default.inner())));
                }
            }

            lines.push(field_def);
        }

        if let Some(pk_columns) = table.primary_key_columns() {
            let pk_fields: Vec<String> = pk_columns.map(|c| format!("{}", Quoted::Backticks(c.name()))).collect();
            lines.push(format!(
                "DEFINE INDEX {table_name}_pk ON TABLE {table_name} FIELDS {fields} UNIQUE",
                table_name = table_name,
                fields = pk_fields.join(", "),
            ));
        }

        lines.join(";\n")
    }

    fn render_drop_enum(&self, _namespace: Option<&str>, _: EnumWalker<'_>) -> Vec<String> {
        unreachable!("SurrealDB does not have enums")
    }

    fn render_drop_foreign_key(&self, _namespace: Option<&str>, _foreign_key: ForeignKeyWalker<'_>) -> String {
        String::new()
    }

    fn render_drop_index(&self, _namespace: Option<&str>, index: IndexWalker<'_>) -> String {
        format!(
            "REMOVE INDEX {index_name} ON TABLE {table_name}",
            index_name = self.quote(index.name()),
            table_name = self.quote(index.table().name()),
        )
    }

    fn render_drop_and_recreate_index(&self, indexes: MigrationPair<IndexWalker<'_>>) -> Vec<String> {
        vec![
            self.render_drop_index(None, indexes.previous),
            self.render_create_index(indexes.next),
        ]
    }

    fn render_drop_table(&self, _namespace: Option<&str>, table_name: &str) -> Vec<String> {
        vec![format!("REMOVE TABLE {}", self.quote(table_name))]
    }

    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();

        for redefine_table in tables {
            let tables = schemas.walk(redefine_table.table_ids);

            result.push(format!("REMOVE TABLE {}", self.quote(tables.previous.name())));
            result.push(self.render_create_table(tables.next));

            for index in tables.next.indexes().filter(|idx| !idx.is_primary_key()) {
                result.push(self.render_create_index(index));
            }
        }

        result
    }

    fn render_rename_table(&self, _namespace: Option<&str>, name: &str, new_name: &str) -> String {
        // SurrealDB does not support RENAME TABLE. This generates a comment
        // that will cause a migration error, forcing manual intervention.
        format!(
            "-- ERROR: SurrealDB does not support RENAME TABLE. Manual migration required: {} -> {}",
            name, new_name
        )
    }

    fn render_drop_view(&self, _namespace: Option<&str>, view: ViewWalker<'_>) -> String {
        format!("REMOVE TABLE {}", self.quote(view.name()))
    }

    fn render_drop_user_defined_type(&self, _namespace: Option<&str>, _: &UserDefinedTypeWalker<'_>) -> String {
        unreachable!("render_drop_user_defined_type on SurrealDB")
    }

    fn render_rename_foreign_key(&self, _fks: MigrationPair<ForeignKeyWalker<'_>>) -> String {
        String::new()
    }
}

fn render_column_type(t: &ColumnType) -> &str {
    match &t.family {
        ColumnTypeFamily::Boolean => "bool",
        ColumnTypeFamily::DateTime => "datetime",
        ColumnTypeFamily::Float => "float",
        ColumnTypeFamily::Decimal => "decimal",
        ColumnTypeFamily::Int => "int",
        // SurrealDB int is 64-bit; BigInt maps directly
        ColumnTypeFamily::BigInt => "int",
        ColumnTypeFamily::String => "string",
        ColumnTypeFamily::Binary => "bytes",
        ColumnTypeFamily::Json => "object",
        ColumnTypeFamily::Enum(_) => "string",
        ColumnTypeFamily::Uuid => "uuid",
        ColumnTypeFamily::Udt(_) => "object",
        ColumnTypeFamily::Unsupported(x) => x.as_ref(),
    }
}

fn render_default(default: &DefaultValue) -> Cow<'_, str> {
    match default.kind() {
        DefaultKind::DbGenerated(Some(val)) => val.as_str().into(),
        DefaultKind::Value(PrismaValue::String(val)) | DefaultKind::Value(PrismaValue::Enum(val)) => {
            format!("'{}'", val.replace('\'', "\\'")).into()
        }
        DefaultKind::Value(PrismaValue::Bytes(b)) => {
            format!("encoding::base64::decode('{}')", base64::Engine::encode(&base64::prelude::BASE64_STANDARD, b)).into()
        }
        DefaultKind::Now => "time::now()".into(),
        DefaultKind::Value(PrismaValue::DateTime(val)) => format!("'{val}'").into(),
        DefaultKind::Value(PrismaValue::Boolean(val)) => {
            if *val { "true".into() } else { "false".into() }
        }
        DefaultKind::Value(val) => val.to_string().into(),
        // UniqueRowid is not supported by SurrealDB (no autoincrement)
        DefaultKind::UniqueRowid => "rand::ulid()".into(),
        DefaultKind::DbGenerated(None) | DefaultKind::Sequence(_) => unreachable!(),
    }
}
