use crate::sql_renderer::{
    IteratorJoin, Quoted, QuotedWithPrefix, SQL_INDENTATION, SqlRenderer, StepRenderer, format_hex, render_nullability,
    render_step,
};
use crate::{
    migration_pair::MigrationPair,
    sql_migration::{
        AlterColumn, AlterEnum, AlterExtension, AlterTable, CreateExtension, DropExtension, ExtensionChange,
        RedefineTable, SequenceChange, SequenceChanges, TableChange,
    },
    sql_schema_differ::{ColumnChange, ColumnChanges},
};
use psl::builtin_connectors::{CockroachType, PostgresType};
use sql_ddl::{
    IndexColumn, SortOrder,
    postgres::{self as ddl, PostgresIdentifier},
};
use sql_schema_describer::{
    ColumnArity, ColumnTypeFamily, DefaultKind, DefaultValue, ForeignKeyAction, PrismaValue, SQLSortOrder, SqlSchema,
    postgres::{PostgresSchemaExt, SqlIndexAlgorithm},
    walkers::*,
};
use std::borrow::Cow;

#[derive(Debug)]
pub struct PostgresRenderer {
    is_cockroach: bool,
}

impl PostgresRenderer {
    pub fn new(is_cockroach: bool) -> Self {
        Self { is_cockroach }
    }

    fn render_column(&self, column: TableColumnWalker<'_>) -> String {
        let column_name = Quoted::postgres_ident(column.name());
        let tpe_str = render_column_type(column, self);
        let nullability_str = render_nullability(column);
        let default_str = column
            .default()
            .map(|d| render_default(d.inner(), &render_column_type(column, self)))
            .filter(|default| !default.is_empty())
            .map(|default| format!(" DEFAULT {default}"))
            .unwrap_or_else(String::new);

        let identity_str = render_column_identity_str(column, self);

        format!("{SQL_INDENTATION}{column_name} {tpe_str}{nullability_str}{default_str}{identity_str}",)
    }
}

impl SqlRenderer for PostgresRenderer {
    // TODO: We only do alter_sequence on CockroachDB.
    fn render_alter_sequence(
        &self,
        sequence_idx: MigrationPair<u32>,
        changes: SequenceChanges,
        schemas: MigrationPair<&SqlSchema>,
    ) -> Vec<String> {
        let exts: MigrationPair<&PostgresSchemaExt> = schemas.map(|schema| schema.downcast_connector_data());
        let (prev_seq, next_seq) = exts
            .zip(sequence_idx)
            .map(|(ext, idx)| &ext.sequences[idx as usize])
            .into_tuple();
        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                stmt.push_str("ALTER SEQUENCE ");
                stmt.push_display(&Quoted::postgres_ident(&prev_seq.name));

                if changes.0.contains(SequenceChange::MinValue) {
                    stmt.push_str(" MINVALUE ");
                    stmt.push_display(&next_seq.min_value);
                }

                if changes.0.contains(SequenceChange::MaxValue) {
                    stmt.push_str(" MAXVALUE ");
                    stmt.push_display(&next_seq.max_value);
                }

                if changes.0.contains(SequenceChange::Increment) {
                    stmt.push_str(" INCREMENT ");
                    stmt.push_display(&next_seq.increment_by);
                }

                if changes.0.contains(SequenceChange::Start) {
                    stmt.push_str(" START ");
                    stmt.push_display(&next_seq.start_value);
                }

                if changes.0.contains(SequenceChange::Cache) {
                    stmt.push_str(" CACHE ");
                    stmt.push_display(&next_seq.cache_size);
                }
            })
        })
    }

    fn render_create_extension(&self, create: &CreateExtension, schema: &SqlSchema) -> Vec<String> {
        let ext: &PostgresSchemaExt = schema.downcast_connector_data();
        let extension = ext.get_extension(create.id);

        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                stmt.push_str("CREATE EXTENSION IF NOT EXISTS ");
                stmt.push_display(&Quoted::postgres_ident(&extension.name));

                if !extension.version.is_empty() || !extension.schema.is_empty() {
                    stmt.push_str(" WITH");
                }

                if !extension.schema.is_empty() {
                    stmt.push_str(" SCHEMA ");
                    stmt.push_display(&Quoted::postgres_ident(&extension.schema));
                }

                if !extension.version.is_empty() {
                    stmt.push_str(" VERSION ");
                    stmt.push_display(&Quoted::postgres_ident(&extension.version));
                }
            })
        })
    }

    fn render_drop_extension(&self, drop: &DropExtension, schema: &SqlSchema) -> Vec<String> {
        let ext: &PostgresSchemaExt = schema.downcast_connector_data();
        let extension = ext.get_extension(drop.id);

        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                stmt.push_str("DROP EXTENSION ");
                stmt.push_display(&Quoted::postgres_ident(&extension.name));
            })
        })
    }

    fn render_alter_extension(&self, alter: &AlterExtension, schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        let exts: MigrationPair<&PostgresSchemaExt> = schemas.map(|schema| schema.downcast_connector_data());
        let extensions = exts.zip(alter.ids).map(|(ext, id)| ext.get_extension(id));

        alter
            .changes
            .iter()
            .flat_map(|change| {
                render_step(&mut |step| match change {
                    ExtensionChange::AlterVersion => step.render_statement(&mut |stmt| {
                        stmt.push_str("ALTER EXTENSION ");
                        stmt.push_display(&Quoted::postgres_ident(&extensions.previous.name));
                        stmt.push_str(" UPDATE TO ");
                        stmt.push_display(&Quoted::postgres_ident(&extensions.next.version));
                    }),
                    ExtensionChange::AlterSchema => step.render_statement(&mut |stmt| {
                        stmt.push_str("ALTER EXTENSION ");
                        stmt.push_display(&Quoted::postgres_ident(&extensions.previous.name));
                        stmt.push_str(" SET SCHEMA ");
                        stmt.push_display(&Quoted::postgres_ident(&extensions.next.schema));
                    }),
                })
            })
            .collect()
    }

    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::postgres_ident(name)
    }

    fn render_add_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String {
        ddl::AlterTable {
            table_name: &QuotedWithPrefix::pg_from_table_walker(foreign_key.table()),
            clauses: vec![ddl::AlterTableClause::AddForeignKey(ddl::ForeignKey {
                constrained_columns: foreign_key.constrained_columns().map(|c| c.name().into()).collect(),
                referenced_columns: foreign_key.referenced_columns().map(|c| c.name().into()).collect(),
                constraint_name: foreign_key.constraint_name().map(From::from),
                referenced_table: &QuotedWithPrefix::pg_from_table_walker(foreign_key.referenced_table()),
                on_delete: Some(match foreign_key.on_delete_action() {
                    ForeignKeyAction::Cascade => ddl::ForeignKeyAction::Cascade,
                    ForeignKeyAction::NoAction => ddl::ForeignKeyAction::NoAction,
                    ForeignKeyAction::Restrict => ddl::ForeignKeyAction::Restrict,
                    ForeignKeyAction::SetDefault => ddl::ForeignKeyAction::SetDefault,
                    ForeignKeyAction::SetNull => ddl::ForeignKeyAction::SetNull,
                }),
                on_update: Some(match foreign_key.on_update_action() {
                    ForeignKeyAction::Cascade => ddl::ForeignKeyAction::Cascade,
                    ForeignKeyAction::NoAction => ddl::ForeignKeyAction::NoAction,
                    ForeignKeyAction::Restrict => ddl::ForeignKeyAction::Restrict,
                    ForeignKeyAction::SetDefault => ddl::ForeignKeyAction::SetDefault,
                    ForeignKeyAction::SetNull => ddl::ForeignKeyAction::SetNull,
                }),
            })],
        }
        .to_string()
    }

    fn render_alter_enum(&self, alter_enum: &AlterEnum, schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        // ALTER TYPE is much more limited on postgres than on cockroachdb.
        //
        // On Postgres:
        // - Values cannot be removed.
        // - Only one value can be added in a single transaction until postgres 11.
        if self.is_cockroach {
            render_step(&mut |step| {
                render_cockroach_alter_enum(alter_enum, schemas, step);
            })
        } else {
            let renderer = self;
            render_postgres_alter_enum(alter_enum, schemas, renderer)
        }
    }

    fn render_alter_primary_key(&self, tables: MigrationPair<TableWalker<'_>>) -> Vec<String> {
        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                stmt.push_str("ALTER TABLE ");
                stmt.push_display(&quoted_alter_table_name(tables));
                stmt.push_str(" ALTER PRIMARY KEY USING COLUMNS (");
                let column_names = tables
                    .next
                    .primary_key()
                    .unwrap() // safe because there is a primary key to alter
                    .column_names()
                    .map(Quoted::postgres_ident);
                stmt.join(", ", column_names);
                stmt.push_str(")");
            })
        })
    }

    fn render_rename_index(&self, indexes: MigrationPair<IndexWalker<'_>>) -> Vec<String> {
        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                let index_previous_name =
                    QuotedWithPrefix::pg_new(indexes.next.table().explicit_namespace(), indexes.previous.name());
                stmt.push_str("ALTER INDEX ");
                stmt.push_str(&index_previous_name.to_string());
                stmt.push_str(" RENAME TO ");
                // Postgres assumes we use the same schema as the previous name's, so we're not
                // allowed to qualify this identifier.
                stmt.push_display(&Quoted::postgres_ident(indexes.next.name()));
            })
        })
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        let AlterTable { changes, table_ids } = alter_table;
        let mut lines = Vec::new();
        let mut before_statements = Vec::new();
        let mut after_statements = Vec::new();
        let tables = schemas.walk(*table_ids);

        for change in changes {
            match change {
                TableChange::DropPrimaryKey => lines.push(format!(
                    "DROP CONSTRAINT {}",
                    Quoted::postgres_ident(tables.previous.primary_key().unwrap().name())
                )),
                TableChange::RenamePrimaryKey => lines.push(format!(
                    "RENAME CONSTRAINT {} TO {}",
                    Quoted::postgres_ident(tables.previous.primary_key().unwrap().name()),
                    Quoted::postgres_ident(tables.next.primary_key().unwrap().name())
                )),
                TableChange::AddPrimaryKey => lines.push({
                    let named = match tables.next.primary_key().map(|pk| pk.name()) {
                        Some(name) => format!("CONSTRAINT {} ", self.quote(name)),
                        None => "".into(),
                    };

                    format!(
                        "ADD {}PRIMARY KEY ({})",
                        named,
                        tables
                            .next
                            .primary_key_columns()
                            .unwrap()
                            .map(|col| self.quote(col.name()))
                            .join(", ")
                    )
                }),
                TableChange::AddColumn {
                    column_id,
                    has_virtual_default: _,
                } => {
                    let column = schemas.next.walk(*column_id);
                    let col_sql = self.render_column(column);

                    lines.push(format!("ADD COLUMN {col_sql}"));
                }
                TableChange::DropColumn { column_id } => {
                    let name = self.quote(schemas.previous.walk(*column_id).name());
                    lines.push(format!("DROP COLUMN {name}"));
                }
                TableChange::AlterColumn(AlterColumn {
                    column_id,
                    changes,
                    type_change: _,
                }) => {
                    let columns = schemas.walk(*column_id);

                    render_alter_column(
                        columns,
                        changes,
                        &mut before_statements,
                        &mut lines,
                        &mut after_statements,
                        self,
                    );
                }
                TableChange::DropAndRecreateColumn { column_id, changes: _ } => {
                    let columns = schemas.walk(*column_id);
                    let name = self.quote(columns.previous.name());

                    lines.push(format!("DROP COLUMN {name}"));

                    let col_sql = self.render_column(columns.next);
                    lines.push(format!("ADD COLUMN {col_sql}"));
                }
            };
        }

        if lines.is_empty() {
            return Vec::new();
        }

        if self.is_cockroach {
            let mut out = Vec::with_capacity(before_statements.len() + after_statements.len() + lines.len());
            out.extend(before_statements);
            for line in lines {
                out.push(format!("ALTER TABLE {} {}", quoted_alter_table_name(tables), line))
            }
            out.extend(after_statements);
            out
        } else {
            let alter_table = format!("ALTER TABLE {} {}", quoted_alter_table_name(tables), lines.join(",\n"));

            before_statements
                .into_iter()
                .chain(std::iter::once(alter_table))
                .chain(after_statements)
                .collect()
        }
    }

    fn render_create_enum(&self, enm: EnumWalker<'_>) -> Vec<String> {
        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                stmt.push_str("CREATE TYPE ");
                stmt.push_display(&QuotedWithPrefix::pg_new(enm.explicit_namespace(), enm.name()));
                stmt.push_str(" AS ENUM (");
                let mut values = enm.values().peekable();
                while let Some(value) = values.next() {
                    stmt.push_display(&Quoted::postgres_string(value));
                    if values.peek().is_some() {
                        stmt.push_str(", ");
                    }
                }
                stmt.push_str(")");
            })
        })
    }

    fn render_create_index(&self, index: IndexWalker<'_>) -> String {
        let pg_ext: &PostgresSchemaExt = index.schema.downcast_connector_data();

        ddl::CreateIndex {
            index_name: index.name().into(),
            is_unique: index.is_unique(),
            table_reference: &QuotedWithPrefix::pg_from_table_walker(index.table()),
            using: Some(match pg_ext.index_algorithm(index.id) {
                SqlIndexAlgorithm::BTree => ddl::IndexAlgorithm::BTree,
                SqlIndexAlgorithm::Hash => ddl::IndexAlgorithm::Hash,
                SqlIndexAlgorithm::Gist => ddl::IndexAlgorithm::Gist,
                SqlIndexAlgorithm::Gin => ddl::IndexAlgorithm::Gin,
                SqlIndexAlgorithm::SpGist => ddl::IndexAlgorithm::SpGist,
                SqlIndexAlgorithm::Brin => ddl::IndexAlgorithm::Brin,
            }),
            columns: index
                .columns()
                .map(|c| IndexColumn {
                    name: c.as_column().name().into(),
                    length: None,
                    sort_order: c.sort_order().map(|so| match so {
                        SQLSortOrder::Asc => SortOrder::Asc,
                        SQLSortOrder::Desc => SortOrder::Desc,
                    }),
                    operator_class: pg_ext.get_opclass(c.id).map(|c| c.kind.as_ref().into()),
                })
                .collect(),
        }
        .to_string()
    }

    fn render_create_namespace(&self, ns: sql_schema_describer::NamespaceWalker<'_>) -> Vec<String> {
        vec![format!(
            "CREATE SCHEMA IF NOT EXISTS {}",
            Quoted::postgres_ident(ns.name())
        )]
    }

    fn render_create_table(&self, table: TableWalker<'_>) -> String {
        self.render_create_table_as(table, QuotedWithPrefix::pg_from_table_walker(table))
    }

    fn render_create_table_as(&self, table: TableWalker<'_>, table_name: QuotedWithPrefix<&str>) -> String {
        let columns: String = table.columns().map(|column| self.render_column(column)).join(",\n");

        let pk = if let Some(pk) = table.primary_key() {
            let named_constraint = format!("CONSTRAINT {} ", Quoted::postgres_ident(pk.name()));

            format!(
                ",\n\n{}{}PRIMARY KEY ({})",
                SQL_INDENTATION,
                named_constraint,
                pk.columns().map(|col| Quoted::postgres_ident(col.name())).join(",")
            )
        } else {
            String::new()
        };

        format!("CREATE TABLE {table_name} (\n{columns}{pk}\n)")
    }

    fn render_drop_enum(&self, dropped_enum: EnumWalker<'_>) -> Vec<String> {
        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                stmt.push_display(&ddl::DropType {
                    type_name: PostgresIdentifier::new(dropped_enum.explicit_namespace(), dropped_enum.name()),
                })
            })
        })
    }

    fn render_drop_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String {
        format!(
            "ALTER TABLE {table} DROP CONSTRAINT {constraint_name}",
            table = PostgresIdentifier::new(foreign_key.table().explicit_namespace(), foreign_key.table().name()),
            constraint_name = Quoted::postgres_ident(foreign_key.constraint_name().unwrap()),
        )
    }

    fn render_drop_index(&self, index: IndexWalker<'_>) -> String {
        ddl::DropIndex {
            index_name: PostgresIdentifier::new(index.table().explicit_namespace(), index.name()),
        }
        .to_string()
    }

    fn render_drop_table(&self, namespace: Option<&str>, table_name: &str) -> Vec<String> {
        render_step(&mut |step| {
            step.render_statement(&mut |stmt| {
                stmt.push_display(&ddl::DropTable {
                    table_name: PostgresIdentifier::new(namespace, table_name),
                    cascade: false,
                })
            })
        })
    }

    fn render_drop_view(&self, view: ViewWalker<'_>) -> String {
        ddl::DropView {
            view_name: PostgresIdentifier::new(view.namespace(), view.name()),
        }
        .to_string()
    }

    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        let mut result = Vec::new();

        for redefine_table in tables {
            let tables = schemas.walk(redefine_table.table_ids);
            let temporary_table_name = format!("_prisma_new_{}", &tables.next.name());
            let quoted_temporary_table = QuotedWithPrefix(
                tables.next.explicit_namespace().map(Quoted::postgres_ident),
                Quoted::postgres_ident(&temporary_table_name),
            );
            result.push(self.render_create_table_as(tables.next, quoted_temporary_table));

            let columns: Vec<_> = redefine_table
                .column_pairs
                .iter()
                .map(|(column_ids, _, _)| schemas.walk(*column_ids).next.name())
                .map(|c| Quoted::postgres_ident(c).to_string())
                .collect();

            let table = tables.previous.name();

            for index in tables.previous.indexes().filter(|idx| !idx.is_primary_key()) {
                result.push(self.render_drop_index(index));
            }

            if !columns.is_empty() {
                let column_names = columns.join(",");
                result.push(format!(
                    r#"INSERT INTO {quoted_temporary_table} ({column_names}) SELECT {column_names} FROM "{table}""#,
                ));
            }

            result.push(
                ddl::DropTable {
                    table_name: PostgresIdentifier::new(tables.previous.explicit_namespace(), tables.previous.name()),
                    cascade: true,
                }
                .to_string(),
            );

            result.push(self.render_rename_table(
                tables.next.explicit_namespace(),
                &temporary_table_name,
                tables.next.name(),
            ));

            for index in tables.next.indexes().filter(|idx| !idx.is_primary_key()) {
                result.push(self.render_create_index(index));
            }

            for fk in tables.next.foreign_keys() {
                result.push(self.render_add_foreign_key(fk));
            }
        }

        result
    }

    fn render_rename_table(&self, namespace: Option<&str>, name: &str, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {}",
            QuotedWithPrefix::pg_new(namespace, name),
            Quoted::postgres_ident(new_name)
        )
    }

    fn render_drop_user_defined_type(&self, _: &UserDefinedTypeWalker<'_>) -> String {
        unreachable!("render_drop_user_defined_type on PostgreSQL")
    }

    fn render_rename_foreign_key(&self, fks: MigrationPair<ForeignKeyWalker<'_>>) -> String {
        format!(
            r#"ALTER TABLE {table} RENAME CONSTRAINT {previous} TO {next}"#,
            table = quoted_alter_table_name(fks.map(ForeignKeyWalker::table)),
            previous = self.quote(fks.previous.constraint_name().unwrap()),
            next = self.quote(fks.next.constraint_name().unwrap()),
        )
    }
}

fn render_column_type(col: TableColumnWalker<'_>, renderer: &PostgresRenderer) -> Cow<'static, str> {
    let t = col.column_type();
    if let Some(enm) = col.column_type_family_as_enum() {
        let name = QuotedWithPrefix::pg_new(enm.explicit_namespace(), enm.name());
        let arity = if t.arity.is_list() { "[]" } else { "" };
        return format!("{name}{arity}").into();
    }

    if let ColumnTypeFamily::Unsupported(description) = &t.family {
        return format!("{}{}", description, if t.arity.is_list() { "[]" } else { "" }).into();
    }

    if renderer.is_cockroach {
        render_column_type_cockroachdb(col)
    } else {
        render_column_type_postgres(col)
    }
}

fn render_column_type_postgres(col: TableColumnWalker<'_>) -> Cow<'static, str> {
    let t = col.column_type();
    let is_autoincrement = col.is_autoincrement();

    let native_type: &PostgresType = col
        .column_native_type()
        .expect("Missing native type in postgres_renderer::render_column_type()");

    let tpe: Cow<'_, str> = match native_type {
        PostgresType::Citext => "CITEXT".into(),
        PostgresType::Oid => "OID".into(),
        PostgresType::Inet => "INET".into(),
        PostgresType::Money => "MONEY".into(),
        PostgresType::SmallInt if is_autoincrement => "SMALLSERIAL".into(),
        PostgresType::SmallInt => "SMALLINT".into(),
        PostgresType::Integer if is_autoincrement => "SERIAL".into(),
        PostgresType::Integer => "INTEGER".into(),
        PostgresType::BigInt if is_autoincrement => "BIGSERIAL".into(),
        PostgresType::BigInt => "BIGINT".into(),
        PostgresType::Decimal(precision) => format!("DECIMAL{}", render_decimal_args(*precision)).into(),
        PostgresType::Real => "REAL".into(),
        PostgresType::DoublePrecision => "DOUBLE PRECISION".into(),
        PostgresType::VarChar(length) => format!("VARCHAR{}", render_optional_args(*length)).into(),
        PostgresType::Char(length) => format!("CHAR{}", render_optional_args(*length)).into(),
        PostgresType::Text => "TEXT".into(),
        PostgresType::ByteA => "BYTEA".into(),
        PostgresType::Date => "DATE".into(),
        PostgresType::Timestamp(precision) => format!("TIMESTAMP{}", render_optional_args(*precision)).into(),
        PostgresType::Timestamptz(precision) => format!("TIMESTAMPTZ{}", render_optional_args(*precision)).into(),
        PostgresType::Time(precision) => format!("TIME{}", render_optional_args(*precision)).into(),
        PostgresType::Timetz(precision) => format!("TIMETZ{}", render_optional_args(*precision)).into(),
        PostgresType::Boolean => "BOOLEAN".into(),
        PostgresType::Bit(length) => format!("BIT{}", render_optional_args(*length)).into(),
        PostgresType::VarBit(length) => format!("VARBIT{}", render_optional_args(*length)).into(),
        PostgresType::Uuid => "UUID".into(),
        PostgresType::Xml => "XML".into(),
        PostgresType::Json => "JSON".into(),
        PostgresType::JsonB => "JSONB".into(),
    };

    if t.arity.is_list() {
        format!("{tpe}[]").into()
    } else {
        tpe
    }
}

fn render_column_type_cockroachdb(col: TableColumnWalker<'_>) -> Cow<'static, str> {
    let t = col.column_type();
    let native_type = col
        .column_native_type()
        .expect("Missing native type in postgres_renderer::render_column_type()");

    let tpe: Cow<'_, str> = match native_type {
        CockroachType::Inet => "INET".into(),
        CockroachType::Int2 => "INT2".into(),
        CockroachType::Int4 => "INT4".into(),
        CockroachType::Int8 => "INT8".into(),
        CockroachType::Oid => "OID".into(),
        CockroachType::Decimal(precision) => format!("DECIMAL{}", render_decimal_args(*precision)).into(),
        CockroachType::Float4 => "FLOAT4".into(),
        CockroachType::Float8 => "FLOAT8".into(),
        CockroachType::String(length) => format!("STRING{}", render_optional_args(*length)).into(),

        // https://www.cockroachlabs.com/docs/stable/string.html
        CockroachType::Char(length) => format!("CHAR{}", render_optional_args(*length)).into(),
        CockroachType::CatalogSingleChar => r#""char""#.into(),

        CockroachType::Bytes => "BYTES".into(),
        CockroachType::Date => "DATE".into(),
        CockroachType::Timestamp(precision) => format!("TIMESTAMP{}", render_optional_args(*precision)).into(),
        CockroachType::Timestamptz(precision) => format!("TIMESTAMPTZ{}", render_optional_args(*precision)).into(),
        CockroachType::Time(precision) => format!("TIME{}", render_optional_args(*precision)).into(),
        CockroachType::Timetz(precision) => format!("TIMETZ{}", render_optional_args(*precision)).into(),
        CockroachType::Bool => "BOOL".into(),
        CockroachType::Bit(length) => format!("BIT{}", render_optional_args(*length)).into(),
        CockroachType::VarBit(length) => format!("VARBIT{}", render_optional_args(*length)).into(),
        CockroachType::Uuid => "UUID".into(),
        CockroachType::JsonB => "JSONB".into(),
    };

    if t.arity.is_list() {
        format!("{tpe}[]").into()
    } else {
        tpe
    }
}

fn render_optional_args(input: Option<u32>) -> String {
    match input {
        None => "".to_string(),
        Some(arg) => format!("({arg})"),
    }
}

fn render_decimal_args(input: Option<(u32, u32)>) -> String {
    match input {
        None => "".to_string(),
        Some((precision, scale)) => format!("({precision},{scale})"),
    }
}

/// Escape an in-memory string so it becomes a valid string literal with default escaping, i.e.
/// replacing `'` characters with `''`.
fn escape_string_literal(s: &str) -> Cow<'_, str> {
    let mut char_indices = s.char_indices();
    let first_idx = loop {
        match char_indices.next() {
            Some((idx, '\'')) => break idx,
            Some(_) => (),
            None => return Cow::Borrowed(s),
        }
    };

    let mut out = String::with_capacity(s.len() + 1); // at least one more char
    out.push_str(&s[0..first_idx]);

    for c in s[first_idx..].chars() {
        match c {
            '\'' => {
                out.push_str("''");
            }
            c => out.push(c),
        }
    }

    Cow::Owned(out)
}

fn render_alter_column(
    columns: MigrationPair<TableColumnWalker<'_>>,
    column_changes: &ColumnChanges,
    before_statements: &mut Vec<String>,
    clauses: &mut Vec<String>,
    after_statements: &mut Vec<String>,
    renderer: &PostgresRenderer,
) {
    let steps = expand_alter_column(columns, column_changes);
    let table_name = quoted_alter_table_name(columns.map(TableColumnWalker::table));
    let column_name = Quoted::postgres_ident(columns.previous.name());

    let alter_column_prefix = format!("ALTER COLUMN {column_name}");

    for step in steps {
        match step {
            PostgresAlterColumn::DropDefault => {
                clauses.push(format!("{} DROP DEFAULT", &alter_column_prefix));

                // We also need to drop the sequence, in case it isn't used by any other column.
                if let Some(DefaultKind::Sequence(sequence_name)) = columns.previous.default().map(|d| d.kind()) {
                    let sequence_is_still_used = columns.next.schema.walk_table_columns().any(|column| matches!(column.default().map(|d| d.kind()), Some(DefaultKind::Sequence(other_sequence)) if other_sequence == sequence_name) && !column.is_same_column(columns.next));

                    if !sequence_is_still_used {
                        after_statements.push(format!("DROP SEQUENCE {}", Quoted::postgres_ident(sequence_name)));
                    }
                }
            }
            PostgresAlterColumn::SetDefault(new_default) => clauses.push(format!(
                "{} SET DEFAULT {}",
                &alter_column_prefix,
                render_default(&new_default, &render_column_type(columns.next, renderer))
            )),
            PostgresAlterColumn::DropNotNull => clauses.push(format!("{} DROP NOT NULL", &alter_column_prefix)),
            PostgresAlterColumn::SetNotNull => clauses.push(format!("{} SET NOT NULL", &alter_column_prefix)),
            PostgresAlterColumn::SetType => clauses.push(format!(
                "{} SET DATA TYPE {}",
                &alter_column_prefix,
                render_column_type(columns.next, renderer)
            )),
            PostgresAlterColumn::AddSequence => {
                // We imitate the sequence that would be automatically created on a `SERIAL` column.
                //
                // See the postgres docs for more details:
                // https://www.postgresql.org/docs/12/datatype-numeric.html#DATATYPE-SERIAL
                let sequence_name = format!(
                    "{namespace}{table_name}_{column_name}_seq",
                    namespace = match columns.next.table().explicit_namespace() {
                        Some(namespace) => format!("{}.", Quoted::postgres_ident(namespace)),
                        None => String::from(""),
                    },
                    table_name = columns.next.table().name(),
                    column_name = columns.next.name()
                )
                .to_lowercase();

                before_statements.push(format!("CREATE SEQUENCE {sequence_name}"));

                clauses.push(format!(
                    "{prefix} SET DEFAULT {default}",
                    prefix = alter_column_prefix,
                    default = format_args!("nextval({})", Quoted::postgres_string(&sequence_name))
                ));

                after_statements.push(format!(
                    "ALTER SEQUENCE {sequence_name} OWNED BY {table_name}.{column_name}",
                ));
            }
        }
    }
}

fn expand_alter_column(
    columns: MigrationPair<TableColumnWalker<'_>>,
    column_changes: &ColumnChanges,
) -> Vec<PostgresAlterColumn> {
    let mut changes = Vec::new();
    let mut set_type = false;

    for change in column_changes.iter() {
        match change {
            ColumnChange::Default => match (columns.previous.default(), columns.next.default()) {
                (_, Some(next_default)) => changes.push(PostgresAlterColumn::SetDefault(next_default.inner().clone())),
                (_, None) => changes.push(PostgresAlterColumn::DropDefault),
            },
            ColumnChange::Arity => match (columns.previous.arity(), columns.next.arity()) {
                (ColumnArity::Required, ColumnArity::Nullable) => changes.push(PostgresAlterColumn::DropNotNull),
                (ColumnArity::Nullable, ColumnArity::Required) => changes.push(PostgresAlterColumn::SetNotNull),
                (ColumnArity::List, ColumnArity::Nullable) => {
                    set_type = true;
                    changes.push(PostgresAlterColumn::DropNotNull)
                }
                (ColumnArity::List, ColumnArity::Required) => {
                    set_type = true;
                    changes.push(PostgresAlterColumn::SetNotNull)
                }
                (ColumnArity::Nullable, ColumnArity::List) | (ColumnArity::Required, ColumnArity::List) => {
                    set_type = true;
                }
                (ColumnArity::Nullable, ColumnArity::Nullable)
                | (ColumnArity::Required, ColumnArity::Required)
                | (ColumnArity::List, ColumnArity::List) => (),
            },
            ColumnChange::TypeChanged => set_type = true,
            ColumnChange::Autoincrement => {
                if columns.previous.is_autoincrement() {
                    // The sequence should be dropped.
                    changes.push(PostgresAlterColumn::DropDefault)
                } else {
                    // The sequence should be created.
                    changes.push(PostgresAlterColumn::AddSequence)
                }
            }
        }
    }

    // This is a flag so we don't push multiple SetTypes from arity and type changes.
    if set_type {
        changes.push(PostgresAlterColumn::SetType);
    }

    changes
}

/// https://www.postgresql.org/docs/9.1/sql-altertable.html
#[derive(Debug)]
enum PostgresAlterColumn {
    SetDefault(sql_schema_describer::DefaultValue),
    DropDefault,
    DropNotNull,
    SetType,
    SetNotNull,
    /// Add an auto-incrementing sequence as a default on the column.
    AddSequence,
}

fn render_default<'a>(default: &'a DefaultValue, full_data_type: &str) -> Cow<'a, str> {
    fn render_constant_default<'a>(value: &'a PrismaValue, full_data_type: &str) -> Cow<'a, str> {
        match value {
            PrismaValue::String(val) | PrismaValue::Enum(val) => format!("'{}'", escape_string_literal(val)).into(),
            PrismaValue::Json(json_value) => {
                let mut out = String::with_capacity(json_value.len() + 2);
                out.push('\'');
                out.push_str(&escape_string_literal(json_value));
                out.push('\'');
                Cow::Owned(out)
            }
            PrismaValue::DateTime(val) => Quoted::postgres_string(val).to_string().into(),
            PrismaValue::Bytes(b) => {
                // https://www.postgresql.org/docs/current/datatype-binary.html
                let mut out = String::with_capacity(b.len() * 2 + 2);
                out.push_str("'\\x");
                format_hex(b, &mut out);
                out.push('\'');
                out.into()
            }
            PrismaValue::List(values) => {
                let mut out = String::new();
                let mut values = values.iter().peekable();

                out.push_str("ARRAY[");

                while let Some(value) = values.next() {
                    out.push_str(render_constant_default(value, full_data_type).as_ref());

                    if values.peek().is_some() {
                        out.push_str(", ");
                    }
                }

                out.push_str("]::");
                out.push_str(full_data_type);

                Cow::Owned(out)
            }

            other => other.to_string().into(),
        }
    }

    match default.kind() {
        DefaultKind::DbGenerated(Some(val)) => Cow::Borrowed(val.as_str()),
        DefaultKind::Value(PrismaValue::String(val)) | DefaultKind::Value(PrismaValue::Enum(val)) => {
            format!("'{}'", escape_string_literal(val)).into()
        }
        DefaultKind::Now => "CURRENT_TIMESTAMP".into(),
        DefaultKind::Value(value) => render_constant_default(value, full_data_type),
        DefaultKind::UniqueRowid => "unique_rowid()".into(),
        DefaultKind::Sequence(_) | DefaultKind::DbGenerated(None) => Default::default(),
    }
}

fn render_postgres_alter_enum(
    alter_enum: &AlterEnum,
    schemas: MigrationPair<&SqlSchema>,
    renderer: &PostgresRenderer,
) -> Vec<String> {
    if alter_enum.dropped_variants.is_empty() {
        let mut stmts: Vec<String> = alter_enum
            .created_variants
            .iter()
            .map(|created_value| {
                format!(
                    "ALTER TYPE {enum_name} ADD VALUE {value}",
                    enum_name = quoted_alter_enum_name(schemas.walk(alter_enum.id)),
                    value = Quoted::postgres_string(created_value)
                )
            })
            .collect();

        if stmts.len() > 1 {
            let warning = indoc::indoc! {
                r#"
                    -- This migration adds more than one value to an enum.
                    -- With PostgreSQL versions 11 and earlier, this is not possible
                    -- in a single migration. This can be worked around by creating
                    -- multiple migrations, each migration adding only one value to
                    -- the enum.
                    "#
            };

            stmts[0] = format!("{}\n\n{}", warning, stmts[0]);
        }

        return stmts;
    }

    let enums = schemas.walk(alter_enum.id);

    let mut stmts = Vec::with_capacity(10);

    let temporary_enum_name = format!("{}_new", &enums.next.name());
    let tmp_name = QuotedWithPrefix::pg_new(enums.next.explicit_namespace(), temporary_enum_name.as_str());
    let tmp_old_name = format!("{}_old", &enums.previous.name());

    stmts.push("BEGIN".to_string());

    // Create the new enum with tmp name
    {
        let create_new_enum = format!(
            "CREATE TYPE {enum_name} AS ENUM ({variants})",
            enum_name = tmp_name,
            variants = enums.next.values().map(Quoted::postgres_string).join(", ")
        );

        stmts.push(create_new_enum);
    }

    // Find all usages as a default and drop them
    {
        for (colid, _) in &alter_enum.previous_usages_as_default {
            let column = schemas.previous.walk(*colid);

            let drop_default = format!(
                r#"ALTER TABLE {table_name} ALTER COLUMN {column_name} DROP DEFAULT"#,
                table_name = QuotedWithPrefix::pg_from_table_walker(column.table()),
                column_name = Quoted::postgres_ident(column.name()),
            );

            stmts.push(drop_default);
        }
    }

    // Alter type of the current columns to new, with a cast
    {
        let affected_columns = schemas.next.walk_table_columns().filter(
            |column| matches!(&column.column_type().family, ColumnTypeFamily::Enum(id) if *id == enums.next.id),
        );

        for column in affected_columns {
            let array = if column.arity().is_list() { "[]" } else { "" };

            let sql = format!(
                "ALTER TABLE {table_name} \
                            ALTER COLUMN {column_name} TYPE {tmp_name}{array} \
                                USING ({column_name}::text::{tmp_name}{array})",
                table_name = QuotedWithPrefix::pg_from_table_walker(column.table()),
                column_name = Quoted::postgres_ident(column.name()),
                array = array,
            );

            stmts.push(sql);
        }
    }

    // Rename old enum
    {
        let sql = format!(
            "ALTER TYPE {enum_name} RENAME TO {tmp_old_name}",
            enum_name = quoted_alter_enum_name(enums),
            tmp_old_name = Quoted::postgres_ident(&tmp_old_name)
        );

        stmts.push(sql);
    }

    // Rename new enum
    {
        let sql = format!(
            "ALTER TYPE {tmp_name} RENAME TO {enum_name}",
            enum_name = Quoted::postgres_ident(enums.next.name())
        );

        stmts.push(sql)
    }

    // Drop old enum
    {
        let sql = ddl::DropType {
            type_name: PostgresIdentifier::new(enums.previous.explicit_namespace(), tmp_old_name.as_str()),
        }
        .to_string();

        stmts.push(sql)
    }

    // Reinstall dropped defaults that need to be reinstalled
    {
        for (columns, next_default) in alter_enum
            .previous_usages_as_default
            .iter()
            .filter_map(|(prev, next)| next.map(|next| schemas.walk(MigrationPair::new(*prev, next))))
            .filter_map(|columns| columns.next.default().map(|next_default| (columns, next_default)))
        {
            let column_name = columns.previous.name();
            let data_type = render_column_type(columns.next, renderer);
            let default_str = render_default(next_default.inner(), &data_type);
            let tables = columns.map(TableColumnWalker::table);

            let set_default = format!(
                "ALTER TABLE {table_name} ALTER COLUMN {column_name} SET DEFAULT {default}",
                table_name = quoted_alter_table_name(tables),
                column_name = Quoted::postgres_ident(&column_name),
                default = default_str,
            );

            stmts.push(set_default);
        }
    }

    stmts.push("COMMIT".to_string());

    stmts
}

fn render_cockroach_alter_enum(
    alter_enum: &AlterEnum,
    schemas: MigrationPair<&SqlSchema>,
    renderer: &mut StepRenderer,
) {
    let enums = schemas.walk(alter_enum.id);
    let mut prefix = String::new();
    prefix.push_str("ALTER TYPE ");
    prefix.push_str(quoted_alter_enum_name(enums).to_string().as_str());

    // Defaults that use a dropped value will need to be recreated after the alter enum.
    let defaults_to_drop = alter_enum
        .previous_usages_as_default
        .iter()
        .filter_map(|(prev_colidx, _)| {
            let col = schemas.previous.walk(*prev_colidx);
            col.default()
                .and_then(|d| d.as_value())
                .and_then(|v| v.as_enum_value())
                .map(|value| (col, value))
        })
        .filter(|(_, value)| !enums.next.values().any(|v| v == *value));

    for (col, _) in defaults_to_drop {
        renderer.render_statement(&mut |stmt| {
            stmt.push_str("ALTER TABLE ");
            stmt.push_display(&QuotedWithPrefix::pg_from_table_walker(col.table()));
            stmt.push_str(" ALTER COLUMN ");
            stmt.push_display(&Quoted::postgres_ident(col.name()));
            stmt.push_str(" DROP DEFAULT");
        })
    }

    for variant in &alter_enum.created_variants {
        renderer.render_statement(&mut |stmt| {
            stmt.push_str(&prefix);
            stmt.push_str(" ADD VALUE '");
            stmt.push_str(variant);
            stmt.push_str("'");
        });
    }

    for variant in &alter_enum.dropped_variants {
        renderer.render_statement(&mut |stmt| {
            stmt.push_str(&prefix);
            stmt.push_str("DROP VALUE '");
            stmt.push_str(variant);
            stmt.push_str("'");
        });
    }
}

fn render_column_identity_str(column: TableColumnWalker<'_>, renderer: &PostgresRenderer) -> String {
    if !renderer.is_cockroach {
        return String::new();
    }

    let sequence = if let Some(seq_name) = column.default().as_ref().and_then(|d| d.as_sequence()) {
        let connector_data: &PostgresSchemaExt = column.schema.downcast_connector_data();
        connector_data
            .sequences
            .iter()
            .find(|sequence| sequence.name == seq_name)
            .unwrap()
    } else {
        return String::new();
    };

    let mut options = Vec::new();

    if sequence.r#virtual {
        options.push("VIRTUAL".to_owned());
    }

    if sequence.increment_by > 1 {
        options.push(format!("INCREMENT {}", sequence.increment_by));
    }

    if sequence.cache_size > 1 {
        options.push(format!("CACHE {}", sequence.cache_size))
    }

    if sequence.start_value > 1 {
        options.push(format!("START {}", sequence.start_value))
    }

    if sequence.min_value > 1 {
        options.push(format!("MINVALUE {}", sequence.min_value))
    }

    if sequence.max_value != 0 && sequence.max_value != i64::MAX {
        options.push(format!("MAXVALUE {}", sequence.max_value))
    }

    if options.is_empty() {
        String::from(" GENERATED BY DEFAULT AS IDENTITY")
    } else {
        format!(" GENERATED BY DEFAULT AS IDENTITY ({})", options.join(" "))
    }
}

/// Quotes a table name for use in an `ALTER TABLE` statement.
/// The namespace is taken from the next schema and only applied if it was explicit
/// (which is valid because we do not support moving tables between schemas).
/// The table name is always taken from the previous schema to handle renames correctly.
fn quoted_alter_table_name(table: MigrationPair<TableWalker<'_>>) -> QuotedWithPrefix<&str> {
    QuotedWithPrefix::pg_new(table.next.explicit_namespace(), table.previous.name())
}

/// Quotes an enum name for use in an `ALTER TYPE` statement.
/// The namespace is taken from the next schema and only applied if it was explicit
/// (which is valid because we do not support moving enums between schemas).
/// The enum name is always taken from the previous schema to handle renames correctly.
fn quoted_alter_enum_name(enm: MigrationPair<EnumWalker<'_>>) -> QuotedWithPrefix<&str> {
    QuotedWithPrefix::pg_new(enm.next.explicit_namespace(), enm.previous.name())
}
