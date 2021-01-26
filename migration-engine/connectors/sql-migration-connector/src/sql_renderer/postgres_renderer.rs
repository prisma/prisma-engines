use super::{common::*, SqlRenderer};
use crate::{
    flavour::PostgresFlavour,
    pair::Pair,
    sql_migration::{AddColumn, AlterColumn, AlterEnum, AlterTable, DropColumn, RedefineTable, TableChange},
    sql_schema_differ::{ColumnChange, ColumnChanges},
};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use sql_ddl::postgres::{self as ddl, CreateEnum, CreateIndex};
use sql_schema_describer::{walkers::*, *};
use std::borrow::Cow;

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
                    ForeignKeyAction::NoAction => ddl::ForeignKeyAction::DoNothing,
                    ForeignKeyAction::Restrict => ddl::ForeignKeyAction::Restrict,
                    ForeignKeyAction::SetDefault => ddl::ForeignKeyAction::SetDefault,
                    ForeignKeyAction::SetNull => ddl::ForeignKeyAction::SetNull,
                }),
                on_update: Some(match foreign_key.on_update_action() {
                    ForeignKeyAction::Cascade => ddl::ForeignKeyAction::Cascade,
                    ForeignKeyAction::NoAction => ddl::ForeignKeyAction::DoNothing,
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
            let stmts: Vec<String> = alter_enum
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

            return stmts;
        }

        let enums = schemas.enums(&alter_enum.index);

        let mut stmts = Vec::with_capacity(10);

        let tmp_name = format!("{}_new", &enums.next().name());
        let tmp_old_name = format!("{}_old", &enums.previous().name());

        stmts.push("BEGIN".to_string());

        // create the new enum with tmp name
        {
            let create_new_enum = format!(
                "CREATE TYPE {enum_name} AS ENUM ({variants})",
                enum_name = Quoted::postgres_ident(&tmp_name),
                variants = enums.next().values().iter().map(Quoted::postgres_string).join(", ")
            );

            stmts.push(create_new_enum);
        }

        // alter type of the current columns to new, with a cast
        {
            let affected_columns = walk_columns(schemas.next()).filter(|column| matches!(&column.column_type().family, ColumnTypeFamily::Enum(name) if name.as_str() == enums.next().name()));

            for column in affected_columns {
                let sql = format!(
                    "ALTER TABLE {schema_name}.{table_name} \
                            ALTER COLUMN {column_name} TYPE {tmp_name} \
                                USING ({column_name}::text::{tmp_name})",
                    schema_name = Quoted::postgres_ident(self.schema_name()),
                    table_name = Quoted::postgres_ident(column.table().name()),
                    column_name = Quoted::postgres_ident(column.name()),
                    tmp_name = Quoted::postgres_ident(&tmp_name),
                );

                stmts.push(sql);
            }
        }

        // rename old enum
        {
            let sql = format!(
                "ALTER TYPE {enum_name} RENAME TO {tmp_old_name}",
                enum_name = Quoted::postgres_ident(enums.previous().name()),
                tmp_old_name = Quoted::postgres_ident(&tmp_old_name)
            );

            stmts.push(sql);
        }

        // rename new enum
        {
            let sql = format!(
                "ALTER TYPE {tmp_name} RENAME TO {enum_name}",
                tmp_name = Quoted::postgres_ident(&tmp_name),
                enum_name = Quoted::postgres_ident(enums.next().name())
            );

            stmts.push(sql)
        }

        // drop old enum
        {
            let sql = format!(
                "DROP TYPE {tmp_old_name}",
                tmp_old_name = Quoted::postgres_ident(&tmp_old_name),
            );

            stmts.push(sql)
        }

        stmts.push("COMMIT".to_string());

        stmts
    }

    fn render_alter_index(&self, indexes: Pair<&IndexWalker<'_>>) -> Vec<String> {
        vec![format!(
            "ALTER INDEX {} RENAME TO {}",
            self.quote(indexes.previous().name()),
            self.quote(indexes.next().name())
        )]
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: &Pair<&SqlSchema>) -> Vec<String> {
        let AlterTable { changes, table_index } = alter_table;

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
                TableChange::AddPrimaryKey { columns } => lines.push(format!(
                    "ADD PRIMARY KEY ({})",
                    columns.iter().map(|colname| self.quote(colname)).join(", ")
                )),
                TableChange::AddColumn(AddColumn { column_index }) => {
                    let column = tables.next().column_at(*column_index);
                    let col_sql = self.render_column(&column);

                    lines.push(format!("ADD COLUMN {}", col_sql));
                }
                TableChange::DropColumn(DropColumn { index }) => {
                    let name = self.quote(tables.previous().column_at(*index).name());
                    lines.push(format!("DROP COLUMN {}", name));
                }
                TableChange::AlterColumn(AlterColumn {
                    column_index,
                    changes,
                    type_change: _,
                }) => {
                    let columns = tables.columns(column_index);

                    render_alter_column(
                        self,
                        &columns,
                        changes,
                        &mut before_statements,
                        &mut lines,
                        &mut after_statements,
                    );
                }
                TableChange::DropAndRecreateColumn {
                    column_index,
                    changes: _,
                } => {
                    let columns = tables.columns(column_index);
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

    fn render_column(&self, column: &ColumnWalker<'_>) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(column);
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .map(|default| self.render_default(default, column.column_type_family()))
            .filter(|default| !default.is_empty())
            .map(|default| format!(" DEFAULT {}", default))
            .unwrap_or_else(String::new);

        format!(
            "{}{} {}{}{}",
            SQL_INDENTATION, column_name, tpe_str, nullability_str, default_str
        )
    }

    fn render_references(&self, foreign_key: &ForeignKeyWalker<'_>) -> String {
        let referenced_columns = foreign_key
            .referenced_column_names()
            .iter()
            .map(Quoted::postgres_ident)
            .join(",");

        format!(
            "REFERENCES {}({}) {} ON UPDATE CASCADE",
            self.quote(&foreign_key.referenced_table().name()),
            referenced_columns,
            render_on_delete(&foreign_key.on_delete_action())
        )
    }

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str> {
        match (default.kind(), family) {
            (DefaultKind::DBGENERATED(val), _) => val.as_str().into(),
            (DefaultKind::VALUE(PrismaValue::String(val)), ColumnTypeFamily::String)
            | (DefaultKind::VALUE(PrismaValue::Enum(val)), ColumnTypeFamily::Enum(_)) => {
                format!("E'{}'", escape_string_literal(&val)).into()
            }
            (DefaultKind::VALUE(PrismaValue::Bytes(b)), ColumnTypeFamily::Binary) => {
                format!("'{}'", format_hex(b)).into()
            }
            (DefaultKind::NOW, ColumnTypeFamily::DateTime) => "CURRENT_TIMESTAMP".into(),
            (DefaultKind::NOW, _) => unreachable!("NOW default on non-datetime column"),
            (DefaultKind::VALUE(val), ColumnTypeFamily::DateTime) => format!("'{}'", val).into(),
            (DefaultKind::VALUE(PrismaValue::String(val)), ColumnTypeFamily::Json) => format!("'{}'", val).into(),
            (DefaultKind::VALUE(val), _) => val.to_string().into(),
            (DefaultKind::SEQUENCE(_), _) => "".into(),
        }
    }

    fn render_create_enum(&self, enm: &EnumWalker<'_>) -> Vec<String> {
        vec![CreateEnum {
            enum_name: enm.name().into(),
            variants: enm.values().iter().map(|s| Cow::Borrowed(s.as_str())).collect(),
        }
        .to_string()]
    }

    fn render_create_index(&self, index: &IndexWalker<'_>) -> String {
        CreateIndex {
            index_name: index.name().into(),
            is_unique: index.index_type().is_unique(),
            table_reference: index.table().name().into(),
            columns: index.columns().map(|c| c.name().into()).collect(),
        }
        .to_string()
    }

    fn render_create_table_as(&self, table: &TableWalker<'_>, table_name: &str) -> String {
        let columns: String = table.columns().map(|column| self.render_column(&column)).join(",\n");

        let primary_columns = table.primary_key_column_names();
        let pk_column_names = primary_columns
            .into_iter()
            .flat_map(|cols| cols.iter())
            .map(|col| self.quote(col))
            .join(",");
        let pk = if !pk_column_names.is_empty() {
            format!(",\n\n{}PRIMARY KEY ({})", SQL_INDENTATION, pk_column_names)
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
        let sql = format!(
            "DROP TYPE {enum_name}",
            enum_name = Quoted::postgres_ident(dropped_enum.name()),
        );

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
        format!("DROP INDEX {}", self.quote(index.name()))
    }

    fn render_drop_table(&self, table_name: &str) -> Vec<String> {
        vec![format!("DROP TABLE {}", self.quote(&table_name))]
    }

    fn render_redefine_tables(&self, _names: &[RedefineTable], _schemas: &Pair<&SqlSchema>) -> Vec<String> {
        unreachable!("render_redefine_table on Postgres")
    }

    fn render_rename_table(&self, name: &str, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {}",
            self.quote(name),
            new_name = self.quote(new_name),
        )
    }
}

pub(crate) fn render_column_type(col: &ColumnWalker<'_>) -> String {
    let t = col.column_type();
    let is_autoincrement = col.is_autoincrement();

    let array = match t.arity {
        ColumnArity::List => "[]",
        _ => "",
    };

    if !t.full_data_type.is_empty() {
        return format!("{}{}", t.full_data_type, array);
    }

    match &t.family {
        ColumnTypeFamily::Boolean => format!("BOOLEAN{}", array),
        ColumnTypeFamily::DateTime => format!("TIMESTAMP(3){}", array),
        ColumnTypeFamily::Float => format!("DECIMAL(65,30){}", array),
        ColumnTypeFamily::Decimal => format!("DECIMAL(65,30){}", array),
        ColumnTypeFamily::Int if is_autoincrement => format!("SERIAL{}", array),
        ColumnTypeFamily::Int => format!("INTEGER{}", array),
        ColumnTypeFamily::BigInt if is_autoincrement => format!("BIGSERIAL{}", array),
        ColumnTypeFamily::BigInt => format!("BIGINT{}", array),
        ColumnTypeFamily::String => format!("TEXT{}", array),
        ColumnTypeFamily::Enum(name) => format!("{}{}", Quoted::postgres_ident(name), array),
        ColumnTypeFamily::Json => format!("JSONB{}", array),
        ColumnTypeFamily::Binary => format!("BYTEA{}", array),
        ColumnTypeFamily::Uuid => unimplemented!("Uuid not handled yet"),
        ColumnTypeFamily::Unsupported(x) => unimplemented!("{} not handled yet", x),
    }
}

fn escape_string_literal(s: &str) -> Cow<'_, str> {
    static STRING_LITERAL_CHARACTER_TO_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'|\\"#).unwrap());

    STRING_LITERAL_CHARACTER_TO_ESCAPE_RE.replace_all(s, "\\$0")
}

fn render_alter_column(
    renderer: &PostgresFlavour,
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
                if let Some(DefaultKind::SEQUENCE(sequence_name)) = columns.previous().default().map(|d| d.kind()) {
                    let sequence_is_still_used = walk_columns(columns.next().schema()).any(|column| matches!(column.default().map(|d| d.kind()), Some(DefaultKind::SEQUENCE(other_sequence)) if other_sequence == sequence_name) && !column.is_same_column(columns.next()));

                    if !sequence_is_still_used {
                        after_statements.push(format!("DROP SEQUENCE {}", Quoted::postgres_ident(sequence_name)));
                    }
                }
            }
            PostgresAlterColumn::SetDefault(new_default) => clauses.push(format!(
                "{} SET DEFAULT {}",
                &alter_column_prefix,
                renderer.render_default(&new_default, columns.next().column_type_family())
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
                    //todo we should probably get rid of the schema here?
                    "ALTER SEQUENCE {sequence_name} OWNED BY {schema_name}.{table_name}.{column_name}",
                    sequence_name = Quoted::postgres_ident(sequence_name),
                    schema_name = Quoted::postgres_ident(renderer.url.schema()),
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
