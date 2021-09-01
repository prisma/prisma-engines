mod alter_table;

use super::{
    common::{self, render_referential_action},
    IteratorJoin, Quoted, SqlRenderer,
};
use crate::{
    flavour::MssqlFlavour,
    pair::Pair,
    sql_migration::{AlterEnum, AlterTable, RedefineTable},
};
use indoc::{formatdoc, indoc};
use native_types::{MsSqlType, MsSqlTypeParameter};
use prisma_value::PrismaValue;
use sql_schema_describer::{
    walkers::{
        ColumnWalker, EnumWalker, ForeignKeyWalker, IndexWalker, TableWalker, UserDefinedTypeWalker, ViewWalker,
    },
    ColumnTypeFamily, DefaultKind, DefaultValue, IndexType, SqlSchema,
};
use std::{
    borrow::Cow,
    fmt::{Display, Write},
};

#[derive(Debug)]
struct QuotedWithSchema<'a> {
    schema_name: &'a str,
    name: &'a str,
}

impl<'a> Display for QuotedWithSchema<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}].[{}]", self.schema_name, self.name)
    }
}

impl MssqlFlavour {
    fn quote_with_schema<'a>(&'a self, name: &'a str) -> QuotedWithSchema<'a> {
        QuotedWithSchema {
            schema_name: self.schema_name(),
            name,
        }
    }

    fn render_column(&self, column: &ColumnWalker<'_>) -> String {
        let column_name = self.quote(column.name());

        let r#type = render_column_type(column);
        let nullability = common::render_nullability(column);

        let default = if column.is_autoincrement() {
            Cow::Borrowed(" IDENTITY(1,1)")
        } else {
            column
                .default()
                .map(|default| {
                    // named constraints
                    let constraint_name = default
                        .constraint_name()
                        .map(Cow::from)
                        // .. or legacy
                        .unwrap_or_else(|| Cow::from(format!("DF__{}__{}", column.table().name(), column.name())));

                    Cow::Owned(format!(
                        " CONSTRAINT {} DEFAULT {}",
                        self.quote(&constraint_name),
                        render_default(default)
                    ))
                })
                .unwrap_or_default()
        };

        format!("{} {}{}{}", column_name, r#type, nullability, default)
    }

    fn render_references(&self, foreign_key: &ForeignKeyWalker<'_>) -> String {
        let cols = foreign_key
            .referenced_column_names()
            .iter()
            .map(Quoted::mssql_ident)
            .join(",");

        format!(
            " REFERENCES {}({}) ON DELETE {} ON UPDATE {}",
            self.quote_with_schema(foreign_key.referenced_table().name()),
            cols,
            render_referential_action(foreign_key.on_delete_action()),
            render_referential_action(foreign_key.on_update_action()),
        )
    }
}

impl SqlRenderer for MssqlFlavour {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::mssql_ident(name)
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: &Pair<&SqlSchema>) -> Vec<String> {
        let AlterTable {
            table_ids: table_index,
            changes,
        } = alter_table;
        let tables = schemas.tables(table_index);

        alter_table::create_statements(self, tables, changes)
    }

    fn render_alter_enum(&self, _: &AlterEnum, _: &Pair<&SqlSchema>) -> Vec<String> {
        unreachable!("render_alter_enum on Microsoft SQL Server")
    }

    fn render_rename_index(&self, indexes: Pair<&IndexWalker<'_>>) -> Vec<String> {
        let index_with_table = format!(
            "{}.{}.{}",
            self.schema_name(),
            indexes.previous().table().name(),
            indexes.previous().name()
        );

        vec![format!(
            "EXEC SP_RENAME N'{index_with_table}', N'{index_new_name}', N'INDEX'",
            index_with_table = index_with_table,
            index_new_name = indexes.next().name(),
        )]
    }

    fn render_create_enum(&self, _: &EnumWalker<'_>) -> Vec<String> {
        unreachable!("render_create_enum on Microsoft SQL Server")
    }

    fn render_create_index(&self, index: &IndexWalker<'_>) -> String {
        let index_type = match index.index_type() {
            IndexType::Unique => "UNIQUE ",
            IndexType::Normal => "",
        };

        let index_name = self.quote(index.name());
        let table_reference = self.quote_with_schema(index.table().name()).to_string();

        let columns = index.columns().map(|c| self.quote(c.name()));

        format!(
            "CREATE {index_type}INDEX {index_name} ON {table_reference}({columns})",
            index_type = index_type,
            index_name = index_name,
            table_reference = table_reference,
            columns = columns.join(", "),
        )
    }

    fn render_create_table_as(&self, table: &TableWalker<'_>, table_name: &str) -> String {
        let columns: String = table
            .columns()
            .map(|column| self.render_column(&column))
            .join(",\n    ");

        let primary_key = if let Some(pk) = table.primary_key() {
            let column_names = pk.columns.iter().map(|col| self.quote(col)).join(",");

            format!(
                ",\n    CONSTRAINT {} PRIMARY KEY ({})",
                self.quote(pk.constraint_name.as_ref().unwrap()),
                column_names
            )
        } else {
            String::new()
        };

        let constraints = table
            .indexes()
            .filter(|index| index.index_type().is_unique())
            .collect::<Vec<_>>();

        let constraints = if !constraints.is_empty() {
            let constraints = constraints
                .iter()
                .map(|index| {
                    let columns = index.columns().map(|col| self.quote(col.name()));

                    format!("CONSTRAINT {} UNIQUE ({})", self.quote(index.name()), columns.join(","))
                })
                .join(",\n    ");

            format!(",\n    {}", constraints)
        } else {
            String::new()
        };

        formatdoc!(
            r#"
            CREATE TABLE {table_name} (
                {columns}{primary_key}{constraints}
            )"#,
            table_name = self.quote_with_schema(table_name),
            columns = columns,
            primary_key = primary_key,
            constraints = constraints,
        )
    }

    fn render_drop_enum(&self, _: &EnumWalker<'_>) -> Vec<String> {
        unreachable!("render_drop_enum on MSSQL")
    }

    fn render_drop_foreign_key(&self, foreign_key: &ForeignKeyWalker<'_>) -> String {
        format!(
            "ALTER TABLE {table} DROP CONSTRAINT {constraint_name}",
            table = self.quote_with_schema(foreign_key.table().name()),
            constraint_name = Quoted::mssql_ident(foreign_key.constraint_name().unwrap()),
        )
    }

    fn render_drop_index(&self, index: &IndexWalker<'_>) -> String {
        match index.index_type() {
            IndexType::Normal => format!(
                "DROP INDEX {} ON {}",
                self.quote(index.name()),
                self.quote_with_schema(index.table().name())
            ),
            IndexType::Unique => format!(
                "ALTER TABLE {} DROP CONSTRAINT {}",
                self.quote_with_schema(index.table().name()),
                self.quote(index.name()),
            ),
        }
    }

    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: &Pair<&SqlSchema>) -> Vec<String> {
        // All needs to be inside a transaction.
        let mut result = vec!["BEGIN TRANSACTION".to_string()];

        for redefine_table in tables {
            let tables = schemas.tables(&redefine_table.table_ids);
            // This is a copy of our new modified table.
            let temporary_table_name = format!("_prisma_new_{}", &tables.next().name());

            // If any of the columns is an identity, we should know about it.
            let needs_autoincrement = redefine_table
                .column_pairs
                .iter()
                .any(|(column_indexes, _, _)| tables.columns(column_indexes).next().is_autoincrement());

            // Let's make the [columns] nicely rendered.
            let columns: Vec<_> = redefine_table
                .column_pairs
                .iter()
                .map(|(column_indexes, _, _)| tables.columns(column_indexes).next().name())
                .map(|c| self.quote(c))
                .map(|c| format!("{}", c))
                .collect();

            let keys = tables.previous().referencing_foreign_keys().filter(|prev| {
                tables
                    .next()
                    .referencing_foreign_keys()
                    .any(|next| prev.foreign_key() == next.foreign_key())
            });

            // We must drop foreign keys pointing to this table before removing
            // any of the table constraints.
            for fk in keys {
                result.push(self.render_drop_foreign_key(&fk));
            }

            // Then the indices...
            for index in tables.previous().indexes() {
                result.push(self.render_drop_index(&index));
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
            "#, table = tables.previous().name(), schema = self.schema_name()});

            // Create the new table.
            result.push(self.render_create_table_as(tables.next(), &temporary_table_name));

            // We cannot insert into autoincrement columns by default. If we
            // have `IDENTITY` in any of the columns, we'll allow inserting
            // momentarily.
            if needs_autoincrement {
                result.push(format!(
                    r#"SET IDENTITY_INSERT {} ON"#,
                    self.quote_with_schema(&temporary_table_name)
                ));
            }

            // Now we copy everything from the old table to the newly created.
            result.push(formatdoc! {r#"
                IF EXISTS(SELECT * FROM {table})
                    EXEC('INSERT INTO {tmp_table} ({columns}) SELECT {columns} FROM {table} WITH (holdlock tablockx)')"#,
                                    columns = columns.join(","),
                                    table = self.quote_with_schema(tables.previous().name()),
                                    tmp_table = self.quote_with_schema(&temporary_table_name),
            });

            // When done copying, disallow identity inserts again if needed.
            if needs_autoincrement {
                result.push(format!(
                    r#"SET IDENTITY_INSERT {} OFF"#,
                    self.quote_with_schema(&temporary_table_name)
                ));
            }

            // Drop the old, now empty table.
            result.extend(self.render_drop_table(tables.previous().name()));

            // Rename the temporary table with the name defined in the migration.
            result.push(self.render_rename_table(&temporary_table_name, tables.next().name()));

            // Recreating all foreign keys pointing to this table
            for fk in tables.next().referencing_foreign_keys() {
                result.push(self.render_add_foreign_key(&fk));
            }

            // Then the indices...
            for index in tables.next().indexes().filter(|i| !i.index_type().is_unique()) {
                result.push(self.render_create_index(&index));
            }
        }

        result.push("COMMIT".to_string());

        result
    }

    fn render_rename_table(&self, name: &str, new_name: &str) -> String {
        let with_schema = format!("{}.{}", self.schema_name(), name);

        format!(
            "EXEC SP_RENAME N{}, N{}",
            Quoted::Single(with_schema),
            Quoted::Single(new_name),
        )
    }

    fn render_add_foreign_key(&self, foreign_key: &ForeignKeyWalker<'_>) -> String {
        let mut add_constraint = String::with_capacity(120);

        write!(
            add_constraint,
            "ALTER TABLE {table} ADD ",
            table = self.quote_with_schema(foreign_key.table().name())
        )
        .unwrap();

        if let Some(constraint_name) = foreign_key.constraint_name() {
            write!(add_constraint, "CONSTRAINT {} ", self.quote(constraint_name)).unwrap();
        } else {
            write!(
                add_constraint,
                "CONSTRAINT [FK__{}__{}] ",
                foreign_key.table().name(),
                foreign_key.constrained_column_names().join("__"),
            )
            .unwrap();
        }

        write!(
            add_constraint,
            "FOREIGN KEY ({})",
            foreign_key
                .constrained_column_names()
                .iter()
                .map(|col| self.quote(col))
                .join(", ")
        )
        .unwrap();

        add_constraint.push_str(&self.render_references(foreign_key));

        add_constraint
    }

    fn render_drop_table(&self, table_name: &str) -> Vec<String> {
        vec![format!("DROP TABLE {}", self.quote_with_schema(table_name))]
    }

    fn render_drop_view(&self, view: &ViewWalker<'_>) -> String {
        format!("DROP VIEW {}", self.quote_with_schema(view.name()))
    }

    fn render_drop_user_defined_type(&self, udt: &UserDefinedTypeWalker<'_>) -> String {
        format!("DROP TYPE {}", self.quote_with_schema(udt.name()))
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

    fn render_rename_foreign_key(&self, fks: &Pair<ForeignKeyWalker<'_>>) -> String {
        format!(
            r#"EXEC sp_rename '{schema}.{previous}', '{next}', 'OBJECT'"#,
            schema = self.schema_name(),
            previous = fks.previous().constraint_name().unwrap(),
            next = fks.next().constraint_name().unwrap(),
        )
    }
}

fn render_column_type(column: &ColumnWalker<'_>) -> Cow<'static, str> {
    fn format_u32_arg(arg: Option<u32>) -> String {
        match arg {
            None => "".to_string(),
            Some(x) => format!("({})", x),
        }
    }
    fn format_type_param(arg: Option<MsSqlTypeParameter>) -> String {
        match arg {
            None => "".to_string(),
            Some(MsSqlTypeParameter::Number(x)) => format!("({})", x),
            Some(MsSqlTypeParameter::Max) => "(max)".to_string(),
        }
    }

    if let ColumnTypeFamily::Unsupported(description) = &column.column_type().family {
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
        MsSqlType::Decimal(Some((p, s))) => format!("DECIMAL({p},{s})", p = p, s = s).into(),
        MsSqlType::Decimal(None) => "DECIMAL".into(),
        MsSqlType::Money => "MONEY".into(),
        MsSqlType::SmallMoney => "SMALLMONEY".into(),
        MsSqlType::Bit => "BIT".into(),
        MsSqlType::Float(bits) => format!("FLOAT{bits}", bits = format_u32_arg(bits)).into(),

        MsSqlType::Real => "REAL".into(),
        MsSqlType::Date => "DATE".into(),
        MsSqlType::Time => "TIME".into(),
        MsSqlType::DateTime => "DATETIME".into(),
        MsSqlType::DateTime2 => "DATETIME2".into(),
        MsSqlType::DateTimeOffset => "DATETIMEOFFSET".into(),
        MsSqlType::SmallDateTime => "SMALLDATETIME".into(),
        MsSqlType::NChar(len) => format!("NCHAR{len}", len = format_u32_arg(len)).into(),
        MsSqlType::Char(len) => format!("CHAR{len}", len = format_u32_arg(len)).into(),
        MsSqlType::VarChar(len) => format!("VARCHAR{len}", len = format_type_param(len)).into(),
        MsSqlType::Text => "TEXT".into(),
        MsSqlType::NVarChar(len) => format!("NVARCHAR{len}", len = format_type_param(len)).into(),
        MsSqlType::NText => "NTEXT".into(),
        MsSqlType::Binary(len) => format!("BINARY{len}", len = format_u32_arg(len)).into(),
        MsSqlType::VarBinary(len) => format!("VARBINARY{len}", len = format_type_param(len)).into(),
        MsSqlType::Image => "IMAGE".into(),
        MsSqlType::Xml => "XML".into(),
        MsSqlType::UniqueIdentifier => "UNIQUEIDENTIFIER".into(),
    }
}

fn escape_string_literal(s: &str) -> String {
    s.replace('\'', r#"''"#)
}

fn render_default(default: &DefaultValue) -> Cow<'_, str> {
    match default.kind() {
        DefaultKind::DbGenerated(val) => val.as_str().into(),
        DefaultKind::Value(PrismaValue::String(val)) | DefaultKind::Value(PrismaValue::Enum(val)) => {
            Quoted::mssql_string(escape_string_literal(val)).to_string().into()
        }
        DefaultKind::Value(PrismaValue::Bytes(b)) => format!("0x{}", common::format_hex(b)).into(),
        DefaultKind::Now => "CURRENT_TIMESTAMP".into(),
        DefaultKind::Value(PrismaValue::DateTime(val)) => Quoted::mssql_string(val).to_string().into(),
        DefaultKind::Value(PrismaValue::Boolean(val)) => Cow::from(if *val { "1" } else { "0" }),
        DefaultKind::Value(val) => val.to_string().into(),
        DefaultKind::Sequence(_) => "".into(),
    }
}
