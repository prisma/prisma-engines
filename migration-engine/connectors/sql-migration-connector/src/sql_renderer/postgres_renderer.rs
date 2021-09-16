use super::{
    common::{format_hex, render_nullability, IteratorJoin, Quoted, SQL_INDENTATION},
    SqlRenderer,
};
use crate::{
    flavour::PostgresFlavour,
    pair::Pair,
    sql_migration::{AlterColumn, AlterEnum, AlterTable, RedefineTable, TableChange},
    sql_schema_differ::{ColumnChange, ColumnChanges},
};
use native_types::PostgresType;
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use sql_ddl::postgres as ddl;
use sql_schema_describer::{
    walkers::*, ColumnArity, ColumnTypeFamily, DefaultKind, DefaultValue, ForeignKeyAction, SqlSchema,
};
use std::borrow::Cow;

impl PostgresFlavour {
    fn render_column(&self, column: &ColumnWalker<'_>) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(column);
        let nullability_str = render_nullability(column);
        let default_str = column
            .default()
            .map(|default| render_default(default))
            .filter(|default| !default.is_empty())
            .map(|default| format!(" DEFAULT {}", default))
            .unwrap_or_else(String::new);

        format!(
            "{}{} {}{}{}",
            SQL_INDENTATION, column_name, tpe_str, nullability_str, default_str
        )
    }
}

impl SqlRenderer for PostgresFlavour {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::postgres_ident(name)
    }

    fn render_add_foreign_key(&self, foreign_key: &ForeignKeyWalker<'_>) -> String {
        ddl::AlterTable {
            table_name: ddl::PostgresIdentifier::Simple(foreign_key.table().name().into()),
            clauses: vec![ddl::AlterTableClause::AddForeignKey(ddl::ForeignKey {
                constrained_columns: foreign_key.constrained_columns().map(|c| c.name().into()).collect(),
                referenced_columns: foreign_key.referenced_column_names().iter().map(|c| c.into()).collect(),
                constraint_name: foreign_key.constraint_name().map(From::from),
                referenced_table: foreign_key.referenced_table().name().into(),
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

    fn render_alter_enum(&self, alter_enum: &AlterEnum, schemas: &Pair<&SqlSchema>) -> Vec<String> {
        if alter_enum.dropped_variants.is_empty() {
            let mut stmts: Vec<String> = alter_enum
                .created_variants
                .iter()
                .map(|created_value| {
                    format!(
                        "ALTER TYPE {enum_name} ADD VALUE {value}",
                        enum_name = Quoted::postgres_ident(schemas.enums(&alter_enum.index).previous().name()),
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

        let enums = schemas.enums(&alter_enum.index);

        let mut stmts = Vec::with_capacity(10);

        let tmp_name = format!("{}_new", &enums.next().name());
        let tmp_old_name = format!("{}_old", &enums.previous().name());

        stmts.push("BEGIN".to_string());

        // Create the new enum with tmp name
        {
            let create_new_enum = format!(
                "CREATE TYPE {enum_name} AS ENUM ({variants})",
                enum_name = Quoted::postgres_ident(&tmp_name),
                variants = enums.next().values().iter().map(Quoted::postgres_string).join(", ")
            );

            stmts.push(create_new_enum);
        }

        // Find all usages as a default and drop them
        {
            for ((table_idx, colidx), _) in &alter_enum.previous_usages_as_default {
                let table = schemas.previous().table_walker_at(*table_idx);

                let drop_default = format!(
                    r#"ALTER TABLE "{table_name}" ALTER COLUMN "{column_name}" DROP DEFAULT"#,
                    table_name = table.name(),
                    column_name = table.column_at(*colidx).name(),
                );

                stmts.push(drop_default);
            }
        }

        // Alter type of the current columns to new, with a cast
        {
            let affected_columns = walk_columns(schemas.next()).filter(|column| matches!(&column.column_type().family, ColumnTypeFamily::Enum(name) if name.as_str() == enums.next().name()));

            for column in affected_columns {
                let array = if column.arity().is_list() { "[]" } else { "" };

                let sql = format!(
                    "ALTER TABLE {table_name} \
                            ALTER COLUMN {column_name} TYPE {tmp_name}{array} \
                                USING ({column_name}::text::{tmp_name}{array})",
                    table_name = Quoted::postgres_ident(column.table().name()),
                    column_name = Quoted::postgres_ident(column.name()),
                    tmp_name = Quoted::postgres_ident(&tmp_name),
                    array = array,
                );

                stmts.push(sql);
            }
        }

        // Rename old enum
        {
            let sql = format!(
                "ALTER TYPE {enum_name} RENAME TO {tmp_old_name}",
                enum_name = Quoted::postgres_ident(enums.previous().name()),
                tmp_old_name = Quoted::postgres_ident(&tmp_old_name)
            );

            stmts.push(sql);
        }

        // Rename new enum
        {
            let sql = format!(
                "ALTER TYPE {tmp_name} RENAME TO {enum_name}",
                tmp_name = Quoted::postgres_ident(&tmp_name),
                enum_name = Quoted::postgres_ident(enums.next().name())
            );

            stmts.push(sql)
        }

        // Drop old enum
        {
            let sql = ddl::DropType {
                type_name: tmp_old_name.as_str().into(),
            }
            .to_string();

            stmts.push(sql)
        }

        // Reinstall dropped defaults that need to be reinstalled
        {
            for ((prev_tblidx, prev_colidx), (next_tblidx, next_colidx)) in alter_enum
                .previous_usages_as_default
                .iter()
                .filter_map(|(prev, next)| next.map(|next| (prev, next)))
            {
                let columns = schemas
                    .tables(&Pair::new(*prev_tblidx, next_tblidx))
                    .columns(&Pair::new(*prev_colidx, next_colidx));

                let table_name = columns.previous().table().name();
                let column_name = columns.previous().name();
                let default_str = columns
                    .next()
                    .default()
                    .and_then(|default| default.as_value())
                    .and_then(|value| value.as_enum_value())
                    .expect("We should only be setting a changed default if there was one on the previous schema and in the next with the same enum.");

                let set_default = format!(
                    "ALTER TABLE {table_name} ALTER COLUMN {column_name} SET DEFAULT '{default}'",
                    table_name = Quoted::postgres_ident(&table_name),
                    column_name = Quoted::postgres_ident(&column_name),
                    default = escape_string_literal(default_str),
                );

                stmts.push(set_default);
            }
        }

        stmts.push("COMMIT".to_string());

        stmts
    }

    fn render_rename_index(&self, indexes: Pair<&IndexWalker<'_>>) -> Vec<String> {
        vec![format!(
            "ALTER INDEX {} RENAME TO {}",
            self.quote(indexes.previous().name()),
            self.quote(indexes.next().name())
        )]
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: &Pair<&SqlSchema>) -> Vec<String> {
        let AlterTable {
            changes,
            table_ids: table_index,
        } = alter_table;

        let mut lines = Vec::new();
        let mut before_statements = Vec::new();
        let mut after_statements = Vec::new();

        let tables = schemas.tables(table_index);

        for change in changes {
            match change {
                TableChange::DropPrimaryKey => lines.push(format!(
                    "DROP CONSTRAINT {}",
                    Quoted::postgres_ident(
                        tables
                            .previous()
                            .primary_key()
                            .and_then(|pk| pk.constraint_name.as_ref())
                            .expect("Missing constraint name for DROP CONSTRAINT on Postgres.")
                    )
                )),
                TableChange::RenamePrimaryKey => lines.push(format!(
                    "RENAME CONSTRAINT {} TO {}",
                    Quoted::postgres_ident(
                        tables
                            .previous()
                            .primary_key()
                            .and_then(|pk| pk.constraint_name.as_ref())
                            .expect("Missing constraint name for DROP CONSTRAINT on Postgres.")
                    ),
                    Quoted::postgres_ident(
                        tables
                            .next()
                            .primary_key()
                            .and_then(|pk| pk.constraint_name.as_ref())
                            .expect("Missing constraint name for DROP CONSTRAINT on Postgres.")
                    )
                )),
                TableChange::AddPrimaryKey => lines.push({
                    let named = match tables.next().primary_key().and_then(|pk| pk.constraint_name.as_ref()) {
                        Some(name) => format!("CONSTRAINT {} ", self.quote(name)),
                        None => "".into(),
                    };

                    format!(
                        "ADD {}PRIMARY KEY ({})",
                        named,
                        tables
                            .next()
                            .primary_key_column_names()
                            .unwrap()
                            .iter()
                            .map(|colname| self.quote(colname))
                            .join(", ")
                    )
                }),
                TableChange::AddColumn { column_id } => {
                    let column = tables.next().column_at(*column_id);
                    let col_sql = self.render_column(&column);

                    lines.push(format!("ADD COLUMN {}", col_sql));
                }
                TableChange::DropColumn { column_id } => {
                    let name = self.quote(tables.previous().column_at(*column_id).name());
                    lines.push(format!("DROP COLUMN {}", name));
                }
                TableChange::AlterColumn(AlterColumn {
                    column_id,
                    changes,
                    type_change: _,
                }) => {
                    let columns = tables.columns(column_id);

                    render_alter_column(
                        &columns,
                        changes,
                        &mut before_statements,
                        &mut lines,
                        &mut after_statements,
                    );
                }
                TableChange::DropAndRecreateColumn { column_id, changes: _ } => {
                    let columns = tables.columns(column_id);
                    let name = self.quote(columns.previous().name());

                    lines.push(format!("DROP COLUMN {}", name));

                    let col_sql = self.render_column(columns.next());
                    lines.push(format!("ADD COLUMN {}", col_sql));
                }
            };
        }

        if lines.is_empty() {
            return Vec::new();
        }

        let alter_table = format!(
            "ALTER TABLE {} {}",
            self.quote(tables.previous().name()),
            lines.join(",\n")
        );

        before_statements
            .into_iter()
            .chain(std::iter::once(alter_table))
            .chain(after_statements.into_iter())
            .collect()
    }

    fn render_create_enum(&self, enm: &EnumWalker<'_>) -> Vec<String> {
        vec![ddl::CreateEnum {
            enum_name: enm.name().into(),
            variants: enm.values().iter().map(|s| Cow::Borrowed(s.as_str())).collect(),
        }
        .to_string()]
    }

    fn render_create_index(&self, index: &IndexWalker<'_>) -> String {
        ddl::CreateIndex {
            index_name: index.name().into(),
            is_unique: index.index_type().is_unique(),
            table_reference: index.table().name().into(),
            columns: index.columns().map(|c| c.name().into()).collect(),
        }
        .to_string()
    }

    fn render_create_table_as(&self, table: &TableWalker<'_>, table_name: &str) -> String {
        let columns: String = table.columns().map(|column| self.render_column(&column)).join(",\n");

        let pk = if let Some(pk) = table.primary_key() {
            let named_constraint = match &pk.constraint_name {
                Some(name) => format!("CONSTRAINT {} ", self.quote(name)),
                None => "".into(),
            };

            format!(
                ",\n\n{}{}PRIMARY KEY ({})",
                SQL_INDENTATION,
                named_constraint,
                pk.columns
                    .as_slice()
                    .iter()
                    .map(|col| self.quote(col.as_ref()))
                    .join(",")
            )
        } else {
            String::new()
        };

        format!(
            "CREATE TABLE {table_name} (\n{columns}{primary_key}\n)",
            table_name = self.quote(table_name),
            columns = columns,
            primary_key = pk,
        )
    }

    fn render_drop_enum(&self, dropped_enum: &EnumWalker<'_>) -> Vec<String> {
        let sql = ddl::DropType {
            type_name: dropped_enum.name().into(),
        }
        .to_string();

        vec![sql]
    }

    fn render_drop_foreign_key(&self, foreign_key: &ForeignKeyWalker<'_>) -> String {
        format!(
            "ALTER TABLE {table} DROP CONSTRAINT {constraint_name}",
            table = self.quote(foreign_key.table().name()),
            constraint_name = Quoted::postgres_ident(foreign_key.constraint_name().unwrap()),
        )
    }

    fn render_drop_index(&self, index: &IndexWalker<'_>) -> String {
        ddl::DropIndex {
            index_name: index.name().into(),
        }
        .to_string()
    }

    fn render_drop_table(&self, table_name: &str) -> Vec<String> {
        vec![ddl::DropTable {
            table_name: table_name.into(),
        }
        .to_string()]
    }

    fn render_drop_view(&self, view: &ViewWalker<'_>) -> String {
        ddl::DropView {
            view_name: view.name().into(),
        }
        .to_string()
    }

    fn render_redefine_tables(&self, _names: &[RedefineTable], _schemas: &Pair<&SqlSchema>) -> Vec<String> {
        unreachable!("render_redefine_table on Postgres")
    }

    fn render_rename_table(&self, name: &str, new_name: &str) -> String {
        ddl::AlterTable {
            table_name: name.into(),
            clauses: vec![ddl::AlterTableClause::RenameTo(new_name.into())],
        }
        .to_string()
    }

    fn render_drop_user_defined_type(&self, _: &UserDefinedTypeWalker<'_>) -> String {
        unreachable!("render_drop_user_defined_type on PostgreSQL")
    }

    fn render_rename_foreign_key(&self, fks: &Pair<ForeignKeyWalker<'_>>) -> String {
        format!(
            r#"ALTER TABLE "{table}" RENAME CONSTRAINT "{previous}" TO "{next}""#,
            table = fks.previous().table().name(),
            previous = fks.previous().constraint_name().unwrap(),
            next = fks.next().constraint_name().unwrap(),
        )
    }
}

pub(crate) fn render_column_type(col: &ColumnWalker<'_>) -> Cow<'static, str> {
    let t = col.column_type();
    let is_autoincrement = col.is_autoincrement();

    if let ColumnTypeFamily::Enum(name) = &t.family {
        return format!("\"{}\"{}", name, if t.arity.is_list() { "[]" } else { "" }).into();
    }

    if let ColumnTypeFamily::Unsupported(description) = &t.family {
        return format!("{}{}", description, if t.arity.is_list() { "[]" } else { "" }).into();
    }

    let native_type = col
        .column_native_type()
        .expect("Missing native type in postgres_renderer::render_column_type()");

    fn render(input: Option<u32>) -> String {
        match input {
            None => "".to_string(),
            Some(arg) => format!("({})", arg),
        }
    }

    fn render_decimal(input: Option<(u32, u32)>) -> String {
        match input {
            None => "".to_string(),
            Some((precision, scale)) => format!("({},{})", precision, scale),
        }
    }

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
        PostgresType::Decimal(precision) => format!("DECIMAL{}", render_decimal(precision)).into(),
        PostgresType::Real => "REAL".into(),
        PostgresType::DoublePrecision => "DOUBLE PRECISION".into(),
        PostgresType::VarChar(length) => format!("VARCHAR{}", render(length)).into(),
        PostgresType::Char(length) => format!("CHAR{}", render(length)).into(),
        PostgresType::Text => "TEXT".into(),
        PostgresType::ByteA => "BYTEA".into(),
        PostgresType::Date => "DATE".into(),
        PostgresType::Timestamp(precision) => format!("TIMESTAMP{}", render(precision)).into(),
        PostgresType::Timestamptz(precision) => format!("TIMESTAMPTZ{}", render(precision)).into(),
        PostgresType::Time(precision) => format!("TIME{}", render(precision)).into(),
        PostgresType::Timetz(precision) => format!("TIMETZ{}", render(precision)).into(),
        PostgresType::Boolean => "BOOLEAN".into(),
        PostgresType::Bit(length) => format!("BIT{}", render(length)).into(),
        PostgresType::VarBit(length) => format!("VARBIT{}", render(length)).into(),
        PostgresType::Uuid => "UUID".into(),
        PostgresType::Xml => "XML".into(),
        PostgresType::Json => "JSON".into(),
        PostgresType::JsonB => "JSONB".into(),
    };

    if t.arity.is_list() {
        format!("{}[]", tpe.to_owned()).into()
    } else {
        tpe
    }
}

fn escape_string_literal(s: &str) -> Cow<'_, str> {
    static STRING_LITERAL_CHARACTER_TO_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'|\\"#).unwrap());

    STRING_LITERAL_CHARACTER_TO_ESCAPE_RE.replace_all(s, "\\$0")
}

fn render_alter_column(
    columns: &Pair<ColumnWalker<'_>>,
    column_changes: &ColumnChanges,
    before_statements: &mut Vec<String>,
    clauses: &mut Vec<String>,
    after_statements: &mut Vec<String>,
) {
    let steps = expand_alter_column(columns, column_changes);
    let table_name = Quoted::postgres_ident(columns.previous().table().name());
    let column_name = Quoted::postgres_ident(columns.previous().name());

    let alter_column_prefix = format!("ALTER COLUMN {}", column_name);

    for step in steps {
        match step {
            PostgresAlterColumn::DropDefault => {
                clauses.push(format!("{} DROP DEFAULT", &alter_column_prefix));

                // We also need to drop the sequence, in case it isn't used by any other column.
                if let Some(DefaultKind::Sequence(sequence_name)) = columns.previous().default().map(|d| d.kind()) {
                    let sequence_is_still_used = walk_columns(columns.next().schema()).any(|column| matches!(column.default().map(|d| d.kind()), Some(DefaultKind::Sequence(other_sequence)) if other_sequence == sequence_name) && !column.is_same_column(columns.next()));

                    if !sequence_is_still_used {
                        after_statements.push(format!("DROP SEQUENCE {}", Quoted::postgres_ident(sequence_name)));
                    }
                }
            }
            PostgresAlterColumn::SetDefault(new_default) => clauses.push(format!(
                "{} SET DEFAULT {}",
                &alter_column_prefix,
                render_default(&new_default)
            )),
            PostgresAlterColumn::DropNotNull => clauses.push(format!("{} DROP NOT NULL", &alter_column_prefix)),
            PostgresAlterColumn::SetNotNull => clauses.push(format!("{} SET NOT NULL", &alter_column_prefix)),
            PostgresAlterColumn::SetType => clauses.push(format!(
                "{} SET DATA TYPE {}",
                &alter_column_prefix,
                render_column_type(columns.next())
            )),
            PostgresAlterColumn::AddSequence => {
                // We imitate the sequence that would be automatically created on a `SERIAL` column.
                //
                // See the postgres docs for more details:
                // https://www.postgresql.org/docs/12/datatype-numeric.html#DATATYPE-SERIAL
                let sequence_name = format!(
                    "{table_name}_{column_name}_seq",
                    table_name = columns.next().table().name(),
                    column_name = columns.next().name()
                )
                .to_lowercase();

                before_statements.push(format!("CREATE SEQUENCE {}", Quoted::postgres_ident(&sequence_name)));

                clauses.push(format!(
                    "{prefix} SET DEFAULT {default}",
                    prefix = alter_column_prefix,
                    default = format_args!("nextval({})", Quoted::postgres_string(&sequence_name))
                ));

                after_statements.push(format!(
                    "ALTER SEQUENCE {sequence_name} OWNED BY {table_name}.{column_name}",
                    sequence_name = Quoted::postgres_ident(sequence_name),
                    table_name = table_name,
                    column_name = column_name,
                ));
            }
        }
    }
}

fn expand_alter_column(columns: &Pair<ColumnWalker<'_>>, column_changes: &ColumnChanges) -> Vec<PostgresAlterColumn> {
    let mut changes = Vec::new();
    let mut set_type = false;

    for change in column_changes.iter() {
        match change {
            ColumnChange::Default => match (columns.previous().default(), columns.next().default()) {
                (_, Some(next_default)) => changes.push(PostgresAlterColumn::SetDefault((*next_default).clone())),
                (_, None) => changes.push(PostgresAlterColumn::DropDefault),
            },
            ColumnChange::Arity => match (columns.previous().arity(), columns.next().arity()) {
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
            ColumnChange::Sequence => {
                if columns.previous().is_autoincrement() {
                    // The sequence should be dropped.
                    changes.push(PostgresAlterColumn::DropDefault)
                } else {
                    // The sequence should be created.
                    changes.push(PostgresAlterColumn::AddSequence)
                }
            }
            ColumnChange::Renaming => unreachable!("column renaming"),
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

fn render_default(default: &DefaultValue) -> Cow<'_, str> {
    match default.kind() {
        DefaultKind::DbGenerated(val) => val.as_str().into(),
        DefaultKind::Value(PrismaValue::String(val)) | DefaultKind::Value(PrismaValue::Enum(val)) => {
            format!("E'{}'", escape_string_literal(val)).into()
        }
        DefaultKind::Value(PrismaValue::Bytes(b)) => Quoted::postgres_string(format_hex(b)).to_string().into(),
        DefaultKind::Now => "CURRENT_TIMESTAMP".into(),
        DefaultKind::Value(PrismaValue::DateTime(val)) => Quoted::postgres_string(val).to_string().into(),
        DefaultKind::Value(val) => val.to_string().into(),
        DefaultKind::Sequence(_) => Default::default(),
    }
}
