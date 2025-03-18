mod alter_table;

use super::{common::*, IteratorJoin, Quoted, SqlRenderer};
use crate::{
    migration_pair::MigrationPair,
    sql_migration::{AlterEnum, AlterTable, RedefineTable},
};
use indoc::{formatdoc, indoc};
use psl::builtin_connectors::{MsSqlType, MsSqlTypeParameter};
use sql_schema_describer::{self as sql, mssql::MssqlSchemaExt, PrismaValue};
use std::{borrow::Cow, fmt::Write};

#[derive(Debug)]
pub struct MssqlRenderer {
    schema_name: String,
}

impl MssqlRenderer {
    pub fn new(schema_name: String) -> Self {
        Self { schema_name }
    }

    fn schema_name(&self) -> &str {
        &self.schema_name
    }

    fn table_name<'a>(&'a self, table: sql::TableWalker<'a>) -> QuotedWithPrefix<&'a str> {
        QuotedWithPrefix(
            Some(Quoted::mssql_ident(
                table.namespace().unwrap_or_else(|| self.schema_name()),
            )),
            Quoted::mssql_ident(table.name()),
        )
    }

    fn quote_with_schema<'a>(&'a self, namespace: Option<&'a str>, name: &'a str) -> QuotedWithPrefix<&'a str> {
        let ns = namespace.unwrap_or_else(|| self.schema_name());
        QuotedWithPrefix(Some(Quoted::mssql_ident(ns)), Quoted::mssql_ident(name))
    }

    fn render_column(&self, column: sql::TableColumnWalker<'_>) -> String {
        let column_name = Quoted::mssql_ident(column.name());

        let r#type = render_column_type(column);
        let nullability = render_nullability(column);

        let default = if column.is_autoincrement() {
            Cow::Borrowed(" IDENTITY(1,1)")
        } else {
            column
                .default()
                .filter(|d| !matches!(d.kind(), sql::DefaultKind::DbGenerated(None)))
                .map(|default| {
                    // named constraints
                    let constraint_name = default
                        .constraint_name()
                        .map(Cow::from)
                        // .. or legacy
                        .unwrap_or_else(|| Cow::from(format!("DF__{}__{}", column.table().name(), column.name())));

                    Cow::Owned(format!(
                        " CONSTRAINT {} DEFAULT {}",
                        Quoted::mssql_ident(&constraint_name),
                        render_default(default.inner())
                    ))
                })
                .unwrap_or_default()
        };

        format!("{column_name} {type}{nullability}{default}")
    }

    fn render_references(&self, foreign_key: sql::ForeignKeyWalker<'_>) -> String {
        let cols = foreign_key
            .referenced_columns()
            .map(|c| Quoted::mssql_ident(c.name()))
            .join(",");

        format!(
            " REFERENCES {}({}) ON DELETE {} ON UPDATE {}",
            self.table_name(foreign_key.referenced_table()),
            cols,
            render_referential_action(foreign_key.on_delete_action()),
            render_referential_action(foreign_key.on_update_action()),
        )
    }
}

impl SqlRenderer for MssqlRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::mssql_ident(name)
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: MigrationPair<&sql::SqlSchema>) -> Vec<String> {
        let AlterTable { table_ids, changes } = alter_table;
        let tables = schemas.walk(*table_ids);
        alter_table::create_statements(self, tables, changes)
    }

    fn render_alter_enum(&self, _: &AlterEnum, _: MigrationPair<&sql::SqlSchema>) -> Vec<String> {
        unreachable!("render_alter_enum on Microsoft SQL Server")
    }

    fn render_rename_index(&self, indexes: MigrationPair<sql::IndexWalker<'_>>) -> Vec<String> {
        let index_with_table = format!(
            "{}.{}.{}",
            indexes
                .previous
                .table()
                .namespace()
                .unwrap_or_else(|| &self.schema_name),
            indexes.previous.table().name(),
            indexes.previous.name()
        );

        vec![format!(
            "EXEC SP_RENAME N'{index_with_table}', N'{index_new_name}', N'INDEX'",
            index_with_table = index_with_table,
            index_new_name = indexes.next.name(),
        )]
    }

    fn render_create_enum(&self, _: sql::EnumWalker<'_>) -> Vec<String> {
        unreachable!("render_create_enum on Microsoft SQL Server")
    }

    fn render_create_index(&self, index: sql::IndexWalker<'_>) -> String {
        let mssql_schema_ext: &MssqlSchemaExt = index.schema.downcast_connector_data();
        let index_name = Quoted::mssql_ident(index.name());
        let table_reference = self.table_name(index.table());

        let columns = index.columns().map(|c| {
            let mut rendered = Quoted::mssql_ident(c.as_column().name()).to_string();

            if let Some(sort_order) = c.sort_order() {
                rendered.push(' ');
                rendered.push_str(sort_order.as_ref());
            }

            rendered
        });

        let clustering = if mssql_schema_ext.index_is_clustered(index.id) {
            "CLUSTERED "
        } else {
            "NONCLUSTERED "
        };

        let columns = columns.join(", ");

        match index.index_type() {
            sql::IndexType::Unique => {
                let constraint_name = Quoted::mssql_ident(index.name());

                format!("ALTER TABLE {table_reference} ADD CONSTRAINT {constraint_name} UNIQUE {clustering}({columns})")
            }
            sql::IndexType::Normal => {
                format!("CREATE {clustering}INDEX {index_name} ON {table_reference}({columns})",)
            }
            sql::IndexType::Fulltext | sql::IndexType::PrimaryKey => unreachable!(),
        }
    }

    fn render_create_table(&self, table: sql::TableWalker<'_>) -> String {
        self.render_create_table_as(table, self.table_name(table))
    }

    fn render_create_table_as(&self, table: sql::TableWalker<'_>, table_name: QuotedWithPrefix<&str>) -> String {
        let columns: String = table.columns().map(|column| self.render_column(column)).join(",\n    ");
        let mssql_schema_ext: &MssqlSchemaExt = table.schema.downcast_connector_data();

        let primary_key = if let Some(pk) = table.primary_key() {
            let column_names = pk
                .columns()
                .map(|col| {
                    let mut rendered = Quoted::mssql_ident(col.name()).to_string();

                    if let Some(sort_order) = col.sort_order() {
                        rendered.push(' ');
                        rendered.push_str(sort_order.as_ref());
                    }

                    rendered
                })
                .join(",");

            let clustering = if mssql_schema_ext.index_is_clustered(pk.id) {
                " CLUSTERED"
            } else {
                " NONCLUSTERED"
            };

            let constraint_name = Quoted::mssql_ident(pk.name());

            format!(",\n    CONSTRAINT {constraint_name} PRIMARY KEY{clustering} ({column_names})",)
        } else {
            String::new()
        };

        let constraints = table.indexes().filter(|index| index.is_unique()).collect::<Vec<_>>();

        let constraints = if !constraints.is_empty() {
            let constraints = constraints
                .iter()
                .map(|index| {
                    let columns = index.columns().map(|col| {
                        let mut rendered = format!("{}", Quoted::mssql_ident(col.as_column().name()));

                        if let Some(sort_order) = col.sort_order() {
                            rendered.push(' ');
                            rendered.push_str(sort_order.as_ref());
                        }

                        rendered
                    });

                    let constraint_name = Quoted::mssql_ident(index.name());
                    let column_names = columns.join(",");

                    let clustering = if mssql_schema_ext.index_is_clustered(index.id) {
                        " CLUSTERED"
                    } else {
                        " NONCLUSTERED"
                    };

                    format!("CONSTRAINT {constraint_name} UNIQUE{clustering} ({column_names})")
                })
                .join(",\n    ");

            format!(",\n    {constraints}")
        } else {
            String::new()
        };

        formatdoc!(
            r#"
            CREATE TABLE {table_name} (
                {columns}{primary_key}{constraints}
            )"#,
        )
    }

    fn render_drop_enum(&self, _: sql::EnumWalker<'_>) -> Vec<String> {
        unreachable!("render_drop_enum on MSSQL")
    }

    fn render_drop_foreign_key(&self, foreign_key: sql::ForeignKeyWalker<'_>) -> String {
        format!(
            "ALTER TABLE {table} DROP CONSTRAINT {constraint_name}",
            table = self.table_name(foreign_key.table()),
            constraint_name = Quoted::mssql_ident(foreign_key.constraint_name().unwrap()),
        )
    }

    fn render_drop_index(&self, index: sql::IndexWalker<'_>) -> String {
        let ext: &MssqlSchemaExt = index.schema.downcast_connector_data();

        if ext.index_is_a_constraint(index.id) {
            format!(
                "ALTER TABLE {} DROP CONSTRAINT {}",
                self.table_name(index.table()),
                Quoted::mssql_ident(index.name()),
            )
        } else {
            format!(
                "DROP INDEX {} ON {}",
                Quoted::mssql_ident(index.name()),
                self.table_name(index.table())
            )
        }
    }

    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: MigrationPair<&sql::SqlSchema>) -> Vec<String> {
        // All needs to be inside a transaction.
        let mut result = vec!["BEGIN TRANSACTION".to_string()];

        for redefine_table in tables {
            let tables = schemas.walk(redefine_table.table_ids);
            // This is a copy of our new modified table.
            let temporary_table_name = format!("_prisma_new_{}", &tables.next.name());

            // If any of the columns is an identity, we should know about it.
            let needs_autoincrement = redefine_table
                .column_pairs
                .iter()
                .any(|(column_indexes, _, _)| schemas.walk(*column_indexes).next.is_autoincrement());

            // Let's make the [columns] nicely rendered.
            let columns: Vec<_> = redefine_table
                .column_pairs
                .iter()
                .map(|(column_indexes, _, _)| schemas.walk(*column_indexes).next.name())
                .map(|c| Quoted::mssql_ident(c).to_string())
                .collect();

            // Drop the indexes on the table.
            for index in tables.previous.indexes().filter(|idx| !idx.is_primary_key()) {
                result.push(self.render_drop_index(index));
            }

            // Remove all constraints from our original table. This will allow
            // us to reuse the same constraint names when creating the temporary
            // table.
            result.push(formatdoc! {r#"
                DECLARE @SQL NVARCHAR(MAX) = N''
                SELECT @SQL += N'ALTER TABLE '
                    + QUOTENAME(OBJECT_SCHEMA_NAME(PARENT_OBJECT_ID))
                    + '.'
                    + QUOTENAME(OBJECT_NAME(PARENT_OBJECT_ID))
                    + ' DROP CONSTRAINT '
                    + OBJECT_NAME(OBJECT_ID) + ';'
                FROM SYS.OBJECTS
                WHERE TYPE_DESC LIKE '%CONSTRAINT'
                    AND OBJECT_NAME(PARENT_OBJECT_ID) = '{table}'
                    AND SCHEMA_NAME(SCHEMA_ID) = '{schema}'
                EXEC sp_executesql @SQL
            "#,
            table = tables.previous.name(),
            schema = tables.previous.namespace().unwrap_or_else(|| self.schema_name())});

            // Create the new table.
            result.push(self.render_create_table_as(
                tables.next,
                self.quote_with_schema(tables.next.namespace(), &temporary_table_name),
            ));

            // We cannot insert into autoincrement columns by default. If we
            // have `IDENTITY` in any of the columns, we'll allow inserting
            // momentarily.
            if needs_autoincrement {
                result.push(format!(
                    r#"SET IDENTITY_INSERT {} ON"#,
                    self.quote_with_schema(tables.next.namespace(), &temporary_table_name)
                ));
            }

            // Now we copy everything from the old table to the newly created.
            result.push(formatdoc! {r#"
                IF EXISTS(SELECT * FROM {table})
                    EXEC('INSERT INTO {tmp_table} ({columns}) SELECT {columns} FROM {table} WITH (holdlock tablockx)')"#,
                                    columns = columns.join(","),
                                    table = self.table_name(tables.previous),
                                    tmp_table = self.quote_with_schema(tables.next.namespace(), &temporary_table_name),
            });

            // When done copying, disallow identity inserts again if needed.
            if needs_autoincrement {
                result.push(format!(
                    r#"SET IDENTITY_INSERT {} OFF"#,
                    self.quote_with_schema(tables.next.namespace(), &temporary_table_name)
                ));
            }

            // Drop the old, now empty table.
            result.extend(self.render_drop_table(tables.previous.namespace(), tables.previous.name()));

            // Rename the temporary table with the name defined in the migration.
            result.push(self.render_rename_table(tables.next.namespace(), &temporary_table_name, tables.next.name()));

            // Recreate the indexes.
            for index in tables.next.indexes().filter(|i| !i.is_unique() && !i.is_primary_key()) {
                result.push(self.render_create_index(index));
            }
        }

        result.push("COMMIT".to_string());

        result
    }

    fn render_rename_table(&self, namespace: Option<&str>, name: &str, new_name: &str) -> String {
        let ns = namespace.unwrap_or_else(|| self.schema_name());
        let with_schema = format!("{ns}.{name}");

        format!(
            "EXEC SP_RENAME N{}, N{}",
            Quoted::Single(with_schema),
            Quoted::Single(new_name),
        )
    }

    fn render_add_foreign_key(&self, foreign_key: sql::ForeignKeyWalker<'_>) -> String {
        let mut add_constraint = String::with_capacity(120);

        write!(
            add_constraint,
            "ALTER TABLE {table} ADD ",
            table = self.table_name(foreign_key.table())
        )
        .unwrap();

        if let Some(constraint_name) = foreign_key.constraint_name() {
            write!(add_constraint, "CONSTRAINT {} ", Quoted::mssql_ident(constraint_name)).unwrap();
        } else {
            write!(
                add_constraint,
                "CONSTRAINT [FK__{}__{}] ",
                foreign_key.table().name(),
                foreign_key.constrained_columns().map(|c| c.name()).join("__"),
            )
            .unwrap();
        }

        write!(
            add_constraint,
            "FOREIGN KEY ({})",
            foreign_key
                .constrained_columns()
                .map(|col| Quoted::mssql_ident(col.name()))
                .join(", ")
        )
        .unwrap();

        add_constraint.push_str(&self.render_references(foreign_key));

        add_constraint
    }

    fn render_drop_table(&self, namespace: Option<&str>, table_name: &str) -> Vec<String> {
        vec![format!("DROP TABLE {}", self.quote_with_schema(namespace, table_name))]
    }

    fn render_drop_view(&self, view: sql::ViewWalker<'_>) -> String {
        format!("DROP VIEW {}", self.quote_with_schema(view.namespace(), view.name()))
    }

    fn render_drop_user_defined_type(&self, udt: &sql::UserDefinedTypeWalker<'_>) -> String {
        format!("DROP TYPE {}", self.quote_with_schema(udt.namespace(), udt.name()))
    }

    fn render_begin_transaction(&self) -> Option<&'static str> {
        let sql = indoc! { r#"
            BEGIN TRY

            BEGIN TRAN;
        "#};

        Some(sql)
    }

    fn render_commit_transaction(&self) -> Option<&'static str> {
        let sql = indoc! { r#"
            COMMIT TRAN;

            END TRY
            BEGIN CATCH

            IF @@TRANCOUNT > 0
            BEGIN
                ROLLBACK TRAN;
            END;
            THROW

            END CATCH
        "# };

        Some(sql)
    }

    fn render_create_namespace(&self, namespace: sql_schema_describer::NamespaceWalker<'_>) -> String {
        format!(
            "EXEC sp_executesql N'CREATE SCHEMA {};';",
            Quoted::mssql_ident(namespace.name())
        )
    }

    fn render_rename_foreign_key(&self, fks: MigrationPair<sql::ForeignKeyWalker<'_>>) -> String {
        format!(
            r#"EXEC sp_rename '{schema}.{previous}', '{next}', 'OBJECT'"#,
            schema = fks.previous.table().namespace().unwrap_or_else(|| self.schema_name()),
            previous = fks.previous.constraint_name().unwrap(),
            next = fks.next.constraint_name().unwrap(),
        )
    }
}

fn render_column_type(column: sql::TableColumnWalker<'_>) -> Cow<'static, str> {
    fn format_u32_arg(arg: Option<u32>) -> String {
        match arg {
            None => "".to_string(),
            Some(x) => format!("({x})"),
        }
    }
    fn format_type_param(arg: Option<MsSqlTypeParameter>) -> String {
        match arg {
            None => "".to_string(),
            Some(MsSqlTypeParameter::Number(x)) => format!("({x})"),
            Some(MsSqlTypeParameter::Max) => "(max)".to_string(),
        }
    }

    if let sql::ColumnTypeFamily::Unsupported(description) = &column.column_type().family {
        return description.to_string().into();
    }

    let native_type = column
        .column_native_type()
        .expect("Missing column native type in mssql_renderer::render_column_type()");

    match native_type {
        MsSqlType::TinyInt => "TINYINT".into(),
        MsSqlType::SmallInt => "SMALLINT".into(),
        MsSqlType::Int => "INT".into(),
        MsSqlType::BigInt => "BIGINT".into(),
        MsSqlType::Decimal(Some((p, s))) => format!("DECIMAL({p},{s})").into(),
        MsSqlType::Decimal(None) => "DECIMAL".into(),
        MsSqlType::Money => "MONEY".into(),
        MsSqlType::SmallMoney => "SMALLMONEY".into(),
        MsSqlType::Bit => "BIT".into(),
        MsSqlType::Float(bits) => format!("FLOAT{bits}", bits = format_u32_arg(*bits)).into(),

        MsSqlType::Real => "REAL".into(),
        MsSqlType::Date => "DATE".into(),
        MsSqlType::Time => "TIME".into(),
        MsSqlType::DateTime => "DATETIME".into(),
        MsSqlType::DateTime2 => "DATETIME2".into(),
        MsSqlType::DateTimeOffset => "DATETIMEOFFSET".into(),
        MsSqlType::SmallDateTime => "SMALLDATETIME".into(),
        MsSqlType::NChar(len) => format!("NCHAR{len}", len = format_u32_arg(*len)).into(),
        MsSqlType::Char(len) => format!("CHAR{len}", len = format_u32_arg(*len)).into(),
        MsSqlType::VarChar(len) => format!("VARCHAR{len}", len = format_type_param(*len)).into(),
        MsSqlType::Text => "TEXT".into(),
        MsSqlType::NVarChar(len) => format!("NVARCHAR{len}", len = format_type_param(*len)).into(),
        MsSqlType::NText => "NTEXT".into(),
        MsSqlType::Binary(len) => format!("BINARY{len}", len = format_u32_arg(*len)).into(),
        MsSqlType::VarBinary(len) => format!("VARBINARY{len}", len = format_type_param(*len)).into(),
        MsSqlType::Image => "IMAGE".into(),
        MsSqlType::Xml => "XML".into(),
        MsSqlType::UniqueIdentifier => "UNIQUEIDENTIFIER".into(),
    }
}

fn escape_string_literal(s: &str) -> String {
    s.replace('\'', r#"''"#)
}

fn render_default(default: &sql::DefaultValue) -> Cow<'_, str> {
    match default.kind() {
        sql::DefaultKind::DbGenerated(val) => val.as_ref().unwrap().as_str().into(),
        sql::DefaultKind::Value(PrismaValue::String(val)) | sql::DefaultKind::Value(PrismaValue::Enum(val)) => {
            Quoted::mssql_string(escape_string_literal(val)).to_string().into()
        }
        sql::DefaultKind::Value(PrismaValue::Bytes(b)) => {
            let mut out = String::with_capacity(b.len() * 2 + 2);
            out.push_str("0x");
            format_hex(b, &mut out);
            out.into()
        }
        sql::DefaultKind::Now => "CURRENT_TIMESTAMP".into(),
        sql::DefaultKind::Value(PrismaValue::DateTime(val)) => Quoted::mssql_string(val).to_string().into(),
        sql::DefaultKind::Value(PrismaValue::Boolean(val)) => Cow::from(if *val { "1" } else { "0" }),
        sql::DefaultKind::Value(val) => val.to_string().into(),
        sql::DefaultKind::Sequence(_) | sql::DefaultKind::UniqueRowid => unreachable!(),
    }
}
