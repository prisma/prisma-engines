mod sql_unexecutable_migration;

use crate::{
    AddColumn, AlterColumn, Component, DropColumn, DropTable, DropTables, SqlError, SqlMigration, SqlMigrationStep,
    SqlResult, TableChange,
};
use migration_connector::{
    ConnectorResult, DestructiveChangeDiagnostics, DestructiveChangesChecker, MigrationWarning, UnexecutableMigration,
};
use quaint::{ast::*, prelude::SqlFamily};
use sql_schema_describer::{ColumnArity, SqlSchema};

pub struct SqlDestructiveChangesChecker<'a> {
    pub connector: &'a crate::SqlMigrationConnector,
}

impl Component for SqlDestructiveChangesChecker<'_> {
    fn connector(&self) -> &crate::SqlMigrationConnector {
        self.connector
    }
}

impl SqlDestructiveChangesChecker<'_> {
    async fn check_table_drop(
        &self,
        table_name: &str,
        diagnostics: &mut DestructiveChangeDiagnostics,
    ) -> SqlResult<()> {
        let rows_count = self.count_rows_in_table(table_name).await?;

        if rows_count > 0 {
            diagnostics.add_warning(MigrationWarning {
                description: format!(
                    "You are about to drop the table `{table_name}`, which is not empty ({rows_count} rows).",
                    table_name = table_name,
                    rows_count = rows_count
                ),
            });
        }

        Ok(())
    }

    async fn count_values_in_column(&self, column_name: &str, table: &sql_schema_describer::Table) -> SqlResult<i64> {
        let query = Select::from_table((self.schema_name(), table.name.as_str()))
            .value(count(quaint::ast::Column::new(column_name)))
            .so_that(column_name.is_not_null());

        let values_count: i64 =
            self.conn()
                .query(query.into())
                .await
                .map_err(SqlError::from)
                .and_then(|result_set| {
                    result_set
                        .first()
                        .as_ref()
                        .and_then(|row| row.at(0))
                        .and_then(|count| count.as_i64())
                        .ok_or_else(|| {
                            SqlError::Generic(anyhow::anyhow!(
                                "Unexpected result set shape when checking dropped columns."
                            ))
                        })
                })?;

        Ok(values_count)
    }

    async fn count_rows_in_table(&self, table_name: &str) -> SqlResult<i64> {
        let query = Select::from_table((self.schema_name(), table_name)).value(count(asterisk()));
        let result_set = self.conn().query(query.into()).await?;
        let rows_count = result_set
            .first()
            .ok_or_else(|| {
                SqlError::Generic(anyhow::anyhow!(
                    "No row was returned when checking for existing rows in the `{}` table.",
                    table_name
                ))
            })?
            .at(0)
            .and_then(|value| value.as_i64())
            .ok_or_else(|| {
                SqlError::Generic(anyhow::anyhow!(
                    "No count was returned when checking for existing rows in the `{}` table.",
                    table_name
                ))
            })?;

        Ok(rows_count)
    }

    /// Emit a warning when we drop a column that contains non-null values.
    async fn check_column_drop(
        &self,
        drop_column: &DropColumn,
        table: &sql_schema_describer::Table,
        diagnostics: &mut DestructiveChangeDiagnostics,
    ) -> SqlResult<()> {
        let values_count = self.count_values_in_column(&drop_column.name, table).await?;

        if values_count > 0 {
            diagnostics.add_warning(MigrationWarning {
                description: format!(
                    "You are about to drop the column `{column_name}` on the `{table_name}` table, which still contains {values_count} non-null values.",
                    column_name=drop_column.name,
                    table_name=&table.name,
                    values_count=values_count,
                )
            })
        }

        Ok(())
    }

    /// Columns cannot be added when all of the following holds:
    ///
    /// - There are existing rows
    /// - The new column is required
    /// - There is no default value for the new column
    async fn check_add_column(
        &self,
        add_column: &AddColumn,
        table: &sql_schema_describer::Table,
        diagnostics: &mut DestructiveChangeDiagnostics,
    ) -> SqlResult<()> {
        let column_is_required_without_default =
            add_column.column.tpe.arity.is_required() && add_column.column.default.is_none();

        // Optional columns and columns with a default can safely be added.
        if !column_is_required_without_default {
            return Ok(());
        }

        let rows_count = self.count_rows_in_table(&table.name).await?;

        // Empty tables can be safely migrated.
        if rows_count == 0 {
            return Ok(());
        }

        let typed_unexecutable = sql_unexecutable_migration::SqlUnexecutableMigration::AddedRequiredFieldToTable {
            column: add_column.column.name.clone(),
            rows_count: Some(rows_count as u64),
            table: table.name.clone(),
        };

        diagnostics.unexecutable_migrations.push(UnexecutableMigration {
            description: format!("{}", typed_unexecutable),
        });

        Ok(())
    }

    /// Are considered safe at the moment:
    ///
    /// - renamings on SQLite
    /// - default changes on SQLite
    /// - Arity changes from required to optional on SQLite
    ///
    /// Are considered unexecutable:
    ///
    /// - Making an optional column required without a default, when there are existing rows in the table.
    async fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        previous_table: &sql_schema_describer::Table,
        diagnostics: &mut DestructiveChangeDiagnostics,
    ) -> SqlResult<()> {
        let previous_column = previous_table
            .column(&alter_column.name)
            .expect("unsupported column renaming");

        let differ = crate::sql_schema_differ::ColumnDiffer {
            previous: previous_column,
            next: &alter_column.column,
        };

        if self.alter_column_is_safe(&differ) {
            return Ok(());
        }

        self.check_for_column_arity_change(&previous_table.name, &differ, diagnostics)
            .await?;

        let values_count = self.count_values_in_column(&alter_column.name, previous_table).await?;

        if values_count > 0 {
            diagnostics.add_warning(MigrationWarning {
                description: format!(
                                 "You are about to alter the column `{column_name}` on the `{table_name}` table, which still contains {values_count} non-null values. The data in that column will be lost.",
                                 column_name=alter_column.name,
                                 table_name=&previous_table.name,
                                 values_count=values_count,
                             )
            })
        } else if previous_table.is_part_of_foreign_key(&alter_column.column.name)
            && alter_column.column.default.is_none()
            && previous_column.default.is_some()
        {
            diagnostics.add_warning(MigrationWarning {
                description: format!(
                    "The migration is about to remove a default value on the foreign key field `{}.{}`.",
                    previous_table.name, alter_column.name,
                ),
            })
        }

        Ok(())
    }

    fn alter_column_is_safe(&self, differ: &crate::sql_schema_differ::ColumnDiffer<'_>) -> bool {
        use crate::sql_migration::expanded_alter_column::*;

        match self.sql_family() {
            SqlFamily::Sqlite => {
                let arity_change_is_safe = match (&differ.previous.tpe.arity, &differ.next.tpe.arity) {
                    // column became required
                    (ColumnArity::Nullable, ColumnArity::Required) => false,
                    // column became nullable
                    (ColumnArity::Required, ColumnArity::Nullable) => true,
                    // nothing changed
                    (ColumnArity::Required, ColumnArity::Required) | (ColumnArity::Nullable, ColumnArity::Nullable) => {
                        true
                    }
                    // not supported on SQLite
                    (ColumnArity::List, _) | (_, ColumnArity::List) => unreachable!(),
                };

                !differ.all_changes().type_changed() && arity_change_is_safe
            }
            SqlFamily::Postgres => {
                let expanded = expand_postgres_alter_column(differ);

                // We keep the match here to keep the exhaustiveness checking for when we add variants.
                if let Some(steps) = expanded {
                    let mut is_safe = true;

                    for step in steps {
                        match step {
                            PostgresAlterColumn::SetDefault(_)
                            | PostgresAlterColumn::DropDefault
                            | PostgresAlterColumn::DropNotNull => (),
                            PostgresAlterColumn::SetType(_) => is_safe = false,
                        }
                    }

                    is_safe
                } else {
                    false
                }
            }
            SqlFamily::Mysql => {
                let expanded = expand_mysql_alter_column(differ);

                // We keep the match here to keep the exhaustiveness checking for when we add variants.
                if let Some(steps) = expanded {
                    let is_safe = true;

                    for step in steps {
                        match step {
                            MysqlAlterColumn::SetDefault(_) | MysqlAlterColumn::DropDefault => (),
                        }
                    }

                    is_safe
                } else {
                    false
                }
            }
        }
    }

    async fn check_for_column_arity_change(
        &self,
        table_name: &str,
        differ: &crate::sql_schema_differ::ColumnDiffer<'_>,
        diagnostics: &mut DestructiveChangeDiagnostics,
    ) -> SqlResult<()> {
        let rows_count = self.count_rows_in_table(table_name).await?;

        if !differ.all_changes().arity_changed()
            || !differ.next.tpe.arity.is_required()
            || rows_count == 0
            || differ.next.default.is_some()
        {
            return Ok(());
        }

        let typed_unexecutable = sql_unexecutable_migration::SqlUnexecutableMigration::MadeOptionalFieldRequired {
            table: table_name.to_owned(),
            column: differ.previous.name.clone(),
        };

        diagnostics.unexecutable_migrations.push(UnexecutableMigration {
            description: format!("{}", typed_unexecutable),
        });

        Ok(())
    }

    async fn check_impl(
        &self,
        steps: &[SqlMigrationStep],
        before: &SqlSchema,
    ) -> SqlResult<DestructiveChangeDiagnostics> {
        let mut diagnostics = DestructiveChangeDiagnostics::new();

        for step in steps {
            match step {
                SqlMigrationStep::AlterTable(alter_table) => {
                    // The table in alter_table is the updated table, but we want to
                    // check against the current state of the table.
                    let before_table = before.get_table(&alter_table.table.name);

                    if let Some(before_table) = before_table {
                        for change in &alter_table.changes {
                            match *change {
                                TableChange::DropColumn(ref drop_column) => {
                                    self.check_column_drop(drop_column, before_table, &mut diagnostics)
                                        .await?
                                }
                                TableChange::AlterColumn(ref alter_column) => {
                                    self.check_alter_column(alter_column, before_table, &mut diagnostics)
                                        .await?
                                }
                                TableChange::AddColumn(ref add_column) => {
                                    self.check_add_column(add_column, before_table, &mut diagnostics)
                                        .await?
                                }
                                _ => (),
                            }
                        }
                    }
                }
                // Here, check for each table we are going to delete if it is empty. If
                // not, return a warning.
                SqlMigrationStep::DropTable(DropTable { name }) => {
                    self.check_table_drop(name, &mut diagnostics).await?;
                }
                SqlMigrationStep::DropTables(DropTables { names }) => {
                    for name in names {
                        self.check_table_drop(name, &mut diagnostics).await?;
                    }
                }
                // SqlMigrationStep::CreateIndex(CreateIndex { table, index }) if index.is_unique() => todo!(),
                // do nothing
                _ => (),
            }
        }

        // Temporary, for better reporting.
        diagnostics.warn_about_unexecutable_migrations();

        Ok(diagnostics)
    }
}

#[async_trait::async_trait]
impl DestructiveChangesChecker<SqlMigration> for SqlDestructiveChangesChecker<'_> {
    async fn check(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        self.check_impl(&database_migration.original_steps, &database_migration.before)
            .await
            .map_err(|sql_error| sql_error.into_connector_error(&self.connection_info()))
    }

    async fn check_unapply(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        self.check_impl(&database_migration.rollback, &database_migration.after)
            .await
            .map_err(|sql_error| sql_error.into_connector_error(&self.connection_info()))
    }
}
