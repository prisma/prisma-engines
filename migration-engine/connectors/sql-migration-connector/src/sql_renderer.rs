mod common;
mod mysql_renderer;
mod postgres_renderer;
mod sqlite_renderer;

pub(crate) use common::{IteratorJoin, Quoted, QuotedWithSchema};

use crate::{
    database_info::DatabaseInfo,
    sql_migration::{
        AddColumn, AlterColumn, AlterEnum, AlterIndex, AlterTable, CreateEnum, CreateIndex, DropColumn, DropEnum,
        DropIndex, TableChange,
    },
    sql_schema_differ::{ColumnDiffer, SqlSchemaDiffer},
};
use quaint::prelude::SqlFamily;
use sql_schema_describer::walkers::{ColumnWalker, TableWalker};
use sql_schema_describer::*;
use std::borrow::Cow;

pub(crate) trait SqlRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str>;

    fn quote_with_schema<'a, 'b>(&'a self, schema_name: &'a str, name: &'b str) -> QuotedWithSchema<'a, &'b str> {
        QuotedWithSchema {
            schema_name,
            name: self.quote(name),
        }
    }

    fn render_alter_enum(
        &self,
        alter_enum: &AlterEnum,
        differ: &SqlSchemaDiffer<'_>,
        schema_name: &str,
    ) -> anyhow::Result<Vec<String>>;

    fn render_column(&self, schema_name: &str, column: ColumnWalker<'_>, add_fk_prefix: bool) -> String;

    fn render_references(&self, schema_name: &str, foreign_key: &ForeignKey) -> String;

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str>;

    /// Attempt to render a database-specific ALTER COLUMN based on the
    /// passed-in differ. `None` means that we could not generate a good (set
    /// of) ALTER COLUMN(s), and we should fall back to dropping and recreating
    /// the column.
    fn render_alter_column(&self, differ: &ColumnDiffer<'_>) -> Option<RenderedAlterColumn>;

    /// Render an `AlterIndex` step.
    fn render_alter_index(
        &self,
        alter_index: &AlterIndex,
        database_info: &DatabaseInfo,
        current_schema: &SqlSchema,
    ) -> anyhow::Result<Vec<String>>;

    fn render_alter_table(
        &self,
        alter_table: &AlterTable,
        database_info: &DatabaseInfo,
        differ: &SqlSchemaDiffer<'_>,
    ) -> Vec<String> {
        let AlterTable { table, changes } = alter_table;
        let schema_name = database_info.connection_info().schema_name();

        let mut lines = Vec::new();
        let mut before_statements = Vec::new();
        let mut after_statements = Vec::new();

        for change in changes {
            match change {
                TableChange::DropPrimaryKey { constraint_name } => match database_info.sql_family() {
                    SqlFamily::Mysql => lines.push("DROP PRIMARY KEY".to_owned()),
                    SqlFamily::Postgres => lines.push(format!(
                        "DROP CONSTRAINT {}",
                        Quoted::postgres_ident(
                            constraint_name
                                .as_ref()
                                .expect("Missing constraint name for DROP CONSTRAINT on Postgres.")
                        )
                    )),
                    _ => (),
                },
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
                    let col_sql = self.render_column(&schema_name, column, true);
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
                    match self.render_alter_column(&column) {
                        Some(RenderedAlterColumn {
                            alter_columns,
                            before,
                            after,
                        }) => {
                            for statement in alter_columns {
                                lines.push(statement);
                            }

                            if let Some(before) = before {
                                before_statements.push(before);
                            }

                            if let Some(after) = after {
                                after_statements.push(after);
                            }
                        }
                        None => {
                            let name = self.quote(&name);
                            lines.push(format!("DROP COLUMN {}", name));

                            let col_sql = self.render_column(&schema_name, column.next, true);
                            lines.push(format!("ADD COLUMN {}", col_sql));
                        }
                    }
                }
            };
        }

        if lines.is_empty() {
            return Vec::new();
        }

        let alter_table = format!(
            "ALTER TABLE {} {}",
            self.quote_with_schema(&schema_name, &table.name),
            lines.join(",\n")
        );

        let statements = before_statements
            .into_iter()
            .chain(std::iter::once(alter_table))
            .chain(after_statements.into_iter())
            .collect();

        statements
    }

    /// Render a `CreateEnum` step.
    fn render_create_enum(&self, create_enum: &CreateEnum) -> Vec<String>;

    /// Render a `CreateIndex` step.
    fn render_create_index(&self, create_index: &CreateIndex, database_info: &DatabaseInfo) -> String;

    /// Render a `CreateTable` step.
    fn render_create_table(&self, table: &TableWalker<'_>, schema_name: &str) -> anyhow::Result<String>;

    /// Render a `DropEnum` step.
    fn render_drop_enum(&self, drop_enum: &DropEnum) -> Vec<String>;

    /// Render a `DropIndex` step.
    fn render_drop_index(&self, drop_index: &DropIndex, database_info: &DatabaseInfo) -> String;

    /// Render a `RedefineTables` step.
    fn render_redefine_tables(
        &self,
        tables: &[String],
        differ: SqlSchemaDiffer<'_>,
        database_info: &DatabaseInfo,
    ) -> Vec<String>;
}

#[derive(Default)]
pub(crate) struct RenderedAlterColumn {
    /// The statements that will be included in the ALTER TABLE
    pub(crate) alter_columns: Vec<String>,
    /// The statements to be run before the ALTER TABLE.
    pub(crate) before: Option<String>,
    /// The statements to be run after the ALTER TABLE.
    pub(crate) after: Option<String>,
}
