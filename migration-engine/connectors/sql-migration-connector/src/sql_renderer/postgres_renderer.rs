use super::{common::*, SqlRenderer};
use crate::{
    database_info::DatabaseInfo,
    flavour::PostgresFlavour,
    sql_migration::{
        expanded_alter_column::{expand_postgres_alter_column, PostgresAlterColumn},
        AddColumn, AddForeignKey, AlterColumn, AlterEnum, AlterIndex, AlterTable, CreateEnum, CreateIndex, DropColumn,
        DropEnum, DropForeignKey, DropIndex, TableChange,
    },
    sql_schema_differ::{ColumnDiffer, SqlSchemaDiffer},
};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use sql_schema_describer::walkers::*;
use sql_schema_describer::*;
use std::{borrow::Cow, fmt::Write as _};

impl PostgresFlavour {
    fn quote_with_schema<'a, 'b>(&'a self, name: &'b str) -> QuotedWithSchema<'a, &'b str> {
        QuotedWithSchema {
            schema_name: self.schema_name(),
            name: self.quote(name),
        }
    }
}

impl SqlRenderer for PostgresFlavour {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::postgres_ident(name)
    }

    fn render_add_foreign_key(&self, add_foreign_key: &AddForeignKey) -> String {
        let AddForeignKey { foreign_key, table } = add_foreign_key;
        let mut add_constraint = String::with_capacity(120);

        write!(
            add_constraint,
            "ALTER TABLE {table} ADD ",
            table = self.quote_with_schema(table)
        )
        .unwrap();

        if let Some(constraint_name) = foreign_key.constraint_name.as_ref() {
            write!(add_constraint, "CONSTRAINT {} ", self.quote(constraint_name)).unwrap();
        }

        write!(
            add_constraint,
            "FOREIGN KEY ({})",
            foreign_key.columns.iter().map(|col| self.quote(col)).join(", ")
        )
        .unwrap();

        add_constraint.push_str(&self.render_references(&table, &foreign_key));

        add_constraint
    }

    fn render_alter_enum(&self, alter_enum: &AlterEnum, differ: &SqlSchemaDiffer<'_>) -> Vec<String> {
        if alter_enum.dropped_variants.is_empty() {
            let stmts: Vec<String> = alter_enum
                .created_variants
                .iter()
                .map(|created_value| {
                    format!(
                        "ALTER TYPE {enum_name} ADD VALUE {value}",
                        enum_name = Quoted::postgres_ident(&alter_enum.name),
                        value = Quoted::postgres_string(created_value)
                    )
                })
                .collect();

            return stmts;
        }

        let new_enum = differ
            .next
            .get_enum(&alter_enum.name)
            .ok_or_else(|| anyhow::anyhow!("Enum `{}` not found in target schema.", alter_enum.name))
            .unwrap();

        let mut stmts = Vec::with_capacity(10);

        let tmp_name = format!("{}_new", &new_enum.name);
        let tmp_old_name = format!("{}_old", &alter_enum.name);

        stmts.push("Begin".to_string());

        // create the new enum with tmp name
        {
            let create_new_enum = format!(
                "CREATE TYPE {enum_name} AS ENUM ({variants})",
                enum_name = QuotedWithSchema {
                    schema_name: self.schema_name(),
                    name: Quoted::postgres_ident(&tmp_name),
                },
                variants = new_enum.values.iter().map(Quoted::postgres_string).join(", ")
            );

            stmts.push(create_new_enum);
        }

        // alter type of the current columns to new, with a cast
        {
            let affected_columns = walk_columns(differ.next).filter(|column| matches!(&column.column_type().family, ColumnTypeFamily::Enum(name) if name.as_str() == alter_enum.name.as_str()));

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
                enum_name = Quoted::postgres_ident(&alter_enum.name),
                tmp_old_name = Quoted::postgres_ident(&tmp_old_name)
            );

            stmts.push(sql);
        }

        // rename new enum
        {
            let sql = format!(
                "ALTER TYPE {tmp_name} RENAME TO {enum_name}",
                tmp_name = Quoted::postgres_ident(&tmp_name),
                enum_name = Quoted::postgres_ident(&new_enum.name)
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

        stmts.push("Commit".to_string());

        stmts
    }

    fn render_alter_index(
        &self,
        alter_index: &AlterIndex,
        _database_info: &DatabaseInfo,
        _current_schema: &SqlSchema,
    ) -> Vec<String> {
        vec![format!(
            "ALTER INDEX {} RENAME TO {}",
            self.quote_with_schema(&alter_index.index_name),
            self.quote(&alter_index.index_new_name)
        )]
    }

    fn render_alter_table(&self, alter_table: &AlterTable, differ: &SqlSchemaDiffer<'_>) -> Vec<String> {
        let AlterTable { table, changes } = alter_table;

        let mut lines = Vec::new();
        let mut before_statements = Vec::new();
        let mut after_statements = Vec::new();

        for change in changes {
            match change {
                TableChange::DropPrimaryKey { constraint_name } => lines.push(format!(
                    "DROP CONSTRAINT {}",
                    Quoted::postgres_ident(
                        constraint_name
                            .as_ref()
                            .expect("Missing constraint name for DROP CONSTRAINT on Postgres.")
                    )
                )),
                TableChange::AddPrimaryKey { columns } => lines.push(format!(
                    "ADD PRIMARY KEY ({})",
                    columns.iter().map(|colname| self.quote(colname)).join(", ")
                )),
                TableChange::AddColumn(AddColumn { column }) => {
                    let column = ColumnWalker {
                        table,
                        schema: differ.next,
                        column,
                    };
                    let col_sql = self.render_column(column);
                    lines.push(format!("ADD COLUMN {}", col_sql));
                }
                TableChange::DropColumn(DropColumn { name }) => {
                    let name = self.quote(&name);
                    lines.push(format!("DROP COLUMN {}", name));
                }
                TableChange::AlterColumn(AlterColumn { name, column: _ }) => {
                    let column = differ
                        .diff_table(&table.name)
                        .expect("AlterTable on unknown table.")
                        .diff_column(name)
                        .expect("AlterColumn on unknown column.");
                    if render_alter_column(self, &column, &mut before_statements, &mut lines, &mut after_statements)
                        .is_none()
                    {
                        let name = self.quote(&name);
                        lines.push(format!("DROP COLUMN {}", name));

                        let col_sql = self.render_column(column.next);
                        lines.push(format!("ADD COLUMN {}", col_sql));
                    }
                }
            };
        }

        if lines.is_empty() {
            return Vec::new();
        }

        let alter_table = format!(
            "ALTER TABLE {} {}",
            self.quote_with_schema(&table.name),
            lines.join(",\n")
        );

        before_statements
            .into_iter()
            .chain(std::iter::once(alter_table))
            .chain(after_statements.into_iter())
            .collect()
    }

    fn render_column(&self, column: ColumnWalker<'_>) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(column.column_type());
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .filter(|default| !matches!(default, DefaultValue::DBGENERATED(_)))
            .map(|default| format!("DEFAULT {}", self.render_default(default, column.column_type_family())))
            .unwrap_or_else(String::new);
        let is_serial = column.is_autoincrement();

        if is_serial {
            format!("{} SERIAL", column_name)
        } else {
            format!("{} {} {} {}", column_name, tpe_str, nullability_str, default_str)
        }
    }

    fn render_references(&self, _table: &str, foreign_key: &ForeignKey) -> String {
        let referenced_columns = foreign_key
            .referenced_columns
            .iter()
            .map(Quoted::postgres_ident)
            .join(",");

        format!(
            "REFERENCES {}({}) {} ON UPDATE CASCADE",
            self.quote_with_schema(&foreign_key.referenced_table),
            referenced_columns,
            render_on_delete(&foreign_key.on_delete_action)
        )
    }

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str> {
        match (default, family) {
            (DefaultValue::DBGENERATED(val), _) => val.as_str().into(),
            (DefaultValue::VALUE(PrismaValue::String(val)), ColumnTypeFamily::String)
            | (DefaultValue::VALUE(PrismaValue::Enum(val)), ColumnTypeFamily::Enum(_)) => {
                format!("E'{}'", escape_string_literal(&val)).into()
            }
            (DefaultValue::NOW, ColumnTypeFamily::DateTime) => "CURRENT_TIMESTAMP".into(),
            (DefaultValue::NOW, _) => unreachable!("NOW default on non-datetime column"),
            (DefaultValue::VALUE(val), ColumnTypeFamily::DateTime) => format!("'{}'", val).into(),
            (DefaultValue::VALUE(PrismaValue::String(val)), ColumnTypeFamily::Json) => format!("'{}'", val).into(),
            (DefaultValue::VALUE(val), _) => val.to_string().into(),
            (DefaultValue::SEQUENCE(_), _) => "".into(),
        }
    }

    fn render_create_enum(&self, create_enum: &CreateEnum) -> Vec<String> {
        let sql = format!(
            r#"CREATE TYPE {enum_name} AS ENUM ({variants})"#,
            enum_name = QuotedWithSchema {
                schema_name: &self.0.schema(),
                name: Quoted::postgres_ident(&create_enum.name)
            },
            variants = create_enum.variants.iter().map(Quoted::postgres_string).join(", "),
        );

        vec![sql]
    }

    fn render_create_index(&self, create_index: &CreateIndex) -> String {
        let Index { name, columns, tpe } = &create_index.index;
        let index_type = match tpe {
            IndexType::Unique => "UNIQUE ",
            IndexType::Normal => "",
        };
        let index_name = self.quote(&name).to_string();
        let table_reference = self.quote_with_schema(&create_index.table).to_string();
        let columns = columns.iter().map(|c| self.quote(c));

        format!(
            "CREATE {index_type}INDEX {index_name} ON {table_reference}({columns})",
            index_type = index_type,
            index_name = index_name,
            table_reference = table_reference,
            columns = columns.join(", ")
        )
    }

    fn render_create_table(&self, table: &TableWalker<'_>) -> String {
        let columns: String = table.columns().map(|column| self.render_column(column)).join(",\n");

        let primary_columns = table.table.primary_key_columns();
        let pk_column_names = primary_columns.iter().map(|col| self.quote(&col)).join(",");
        let pk = if !pk_column_names.is_empty() {
            format!(",\nPRIMARY KEY ({})", pk_column_names)
        } else {
            String::new()
        };

        format!(
            "CREATE TABLE {table_name} (\n{columns}{primary_key}\n)",
            table_name = self.quote_with_schema(table.name()),
            columns = columns,
            primary_key = pk,
        )
    }

    fn render_drop_enum(&self, drop_enum: &DropEnum) -> Vec<String> {
        let sql = format!(
            "DROP TYPE {enum_name}",
            enum_name = Quoted::postgres_ident(&drop_enum.name),
        );

        vec![sql]
    }

    fn render_drop_foreign_key(&self, drop_foreign_key: &DropForeignKey) -> String {
        format!(
            "ALTER TABLE {table} DROP CONSTRAINT {constraint_name}",
            table = self.quote_with_schema(&drop_foreign_key.table),
            constraint_name = Quoted::postgres_ident(&drop_foreign_key.constraint_name),
        )
    }

    fn render_drop_index(&self, drop_index: &DropIndex) -> String {
        format!("DROP INDEX {}", self.quote_with_schema(&drop_index.name))
    }

    fn render_drop_table(&self, table_name: &str) -> Vec<String> {
        vec![format!("DROP TABLE {}", self.quote_with_schema(&table_name))]
    }

    fn render_redefine_tables(&self, _names: &[String], _differ: SqlSchemaDiffer<'_>) -> Vec<String> {
        unreachable!("render_redefine_table on Postgres")
    }

    fn render_rename_table(&self, name: &str, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {}",
            self.quote_with_schema(&name),
            new_name = self.quote_with_schema(&new_name).to_string(),
        )
    }
}

pub(crate) fn render_column_type(t: &ColumnType) -> String {
    let array = match t.arity {
        ColumnArity::List => "[]",
        _ => "",
    };

    if !t.full_data_type.is_empty() {
        return format!("{}{}", t.full_data_type, array);
    }

    match &t.family {
        ColumnTypeFamily::Boolean => format!("boolean {}", array),
        ColumnTypeFamily::DateTime => format!("timestamp(3) {}", array),
        ColumnTypeFamily::Float => format!("Decimal(65,30) {}", array),
        ColumnTypeFamily::Int => format!("integer {}", array),
        ColumnTypeFamily::String => format!("text {}", array),
        ColumnTypeFamily::Enum(name) => format!("{}{}", Quoted::postgres_ident(name), array),
        ColumnTypeFamily::Json => format!("jsonb {}", array),
        ColumnTypeFamily::Duration => unimplemented!("Duration not handled yet"),
        ColumnTypeFamily::Decimal => unimplemented!("Decimal not handled yet"),
        ColumnTypeFamily::Binary => unimplemented!("Binary not handled yet"),
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
    differ: &ColumnDiffer<'_>,
    before_statements: &mut Vec<String>,
    clauses: &mut Vec<String>,
    after_statements: &mut Vec<String>,
) -> Option<()> {
    // Matches the sequence name from inside an autoincrement default expression.
    static SEQUENCE_DEFAULT_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"nextval\('"?([^"]+)"?'::regclass\)"#).unwrap());

    let steps = expand_postgres_alter_column(differ)?;
    let table_name = Quoted::postgres_ident(differ.previous.table().name());
    let column_name = Quoted::postgres_ident(differ.previous.name());

    let alter_column_prefix = format!("ALTER COLUMN {}", column_name);

    for step in steps {
        match step {
            PostgresAlterColumn::DropDefault => {
                clauses.push(format!("{} DROP DEFAULT", &alter_column_prefix));

                // We also need to drop the sequence, in case it isn't used by any other column.
                if let Some(DefaultValue::SEQUENCE(sequence_expression)) = differ.previous.default() {
                    let sequence_name = SEQUENCE_DEFAULT_RE
                        .captures(sequence_expression)
                        .and_then(|captures| captures.get(1))
                        .map(|capture| capture.as_str())
                        .unwrap_or_else(|| panic!("Failed to extract sequence name from `{}`", sequence_expression));

                    let sequence_is_still_used = walk_columns(differ.next.schema()).any(|column| matches!(column.default(), Some(DefaultValue::SEQUENCE(other_sequence)) if other_sequence == sequence_expression) && !column.is_same_column(&differ.next));

                    if !sequence_is_still_used {
                        after_statements.push(format!("DROP SEQUENCE {}", Quoted::postgres_ident(sequence_name)));
                    }
                }
            }
            PostgresAlterColumn::SetDefault(new_default) => clauses.push(format!(
                "{} SET DEFAULT {}",
                &alter_column_prefix,
                renderer.render_default(&new_default, differ.next.column_type_family())
            )),
            PostgresAlterColumn::DropNotNull => clauses.push(format!("{} DROP NOT NULL", &alter_column_prefix)),
            PostgresAlterColumn::SetNotNull => clauses.push(format!("{} SET NOT NULL", &alter_column_prefix)),
            PostgresAlterColumn::SetType(ty) => clauses.push(format!(
                "{} SET DATA TYPE {}",
                &alter_column_prefix,
                render_column_type(&ty)
            )),
            PostgresAlterColumn::AddSequence => {
                // We imitate the sequence that would be automatically created on a `SERIAL` column.
                //
                // See the postgres docs for more details:
                // https://www.postgresql.org/docs/12/datatype-numeric.html#DATATYPE-SERIAL
                let sequence_name = format!(
                    "{table_name}_{column_name}_seq",
                    table_name = differ.next.table().name(),
                    column_name = differ.next.name()
                )
                .to_lowercase();

                before_statements.push(format!("CREATE SEQUENCE {}", Quoted::postgres_ident(&sequence_name)));

                clauses.push(format!(
                    "{prefix} SET DEFAULT {default}",
                    prefix = alter_column_prefix,
                    default = format_args!("nextval({})", Quoted::postgres_string(&sequence_name))
                ));

                after_statements.push(format!(
                    "ALTER SEQUENCE {sequence_name} OWNED BY {schema_name}.{table_name}.{column_name}",
                    sequence_name = Quoted::postgres_ident(sequence_name),
                    schema_name = Quoted::postgres_ident(renderer.0.schema()),
                    table_name = table_name,
                    column_name = column_name,
                ));
            }
        }
    }

    Some(())
}
