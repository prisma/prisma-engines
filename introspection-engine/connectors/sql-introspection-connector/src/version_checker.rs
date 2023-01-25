//! Prisma version information lookup.

use crate::datamodel_calculator::InputContext;
use crate::introspection_helpers::{
    has_created_at_and_updated_at, is_new_migration_table, is_old_migration_table, is_prisma_1_or_11_list_table,
    is_prisma_1_point_0_join_table, is_prisma_1_point_1_or_2_join_table, is_relay_table,
};
use crate::SqlFamilyTrait;
use introspection_connector::Version;
use psl::builtin_connectors::{MySqlType, PostgresType};
use quaint::connector::SqlFamily;
use sql_schema_describer::ForeignKeyWalker;
use sql_schema_describer::{
    walkers::{ColumnWalker, TableWalker},
    ForeignKeyAction,
};

/// A list of indicators to be used determining Prisma version.
#[derive(Debug)]
struct VersionChecker {
    sql_family: SqlFamily,
    is_cockroachdb: bool,
    has_migration_table: bool,
    has_relay_table: bool,
    has_prisma_1_join_table: bool,
    has_prisma_1_1_or_2_join_table: bool,
    uses_on_delete: bool,
    uses_default_values: bool,
    always_has_created_at_updated_at: bool,
    always_has_p1_or_p_1_1_compatible_id: bool,
    uses_non_prisma_types: bool,
    has_inline_relations: bool,
}

/// Allowed types in SQLite.
const SQLITE_TYPES: &[&str] = &["boolean", "date", "real", "integer", "text"];

/// Allowed types in PostgreSQL.
const POSTGRES_TYPES: &[PostgresType] = &[
    PostgresType::Boolean,
    PostgresType::Timestamp(Some(3)),
    PostgresType::Decimal(Some((65, 30))),
    PostgresType::Integer,
    PostgresType::Text,
    PostgresType::VarChar(Some(25)),
    PostgresType::VarChar(Some(36)),
    PostgresType::VarChar(Some(191)),
];

/// Allowed types in MySQL.
const MYSQL_TYPES: &[MySqlType] = &[
    MySqlType::TinyInt,
    MySqlType::DateTime(Some(3)),
    MySqlType::Decimal(Some((65, 30))),
    MySqlType::Int,
    MySqlType::MediumText,
    MySqlType::VarChar(191),
    MySqlType::Char(25),
    MySqlType::Char(36),
];

/// Find out if the database is created with a specific version of
/// Prisma.
pub(crate) fn check_prisma_version(input: &InputContext<'_>) -> Version {
    let mut version_checker = VersionChecker {
        sql_family: input.sql_family(),
        is_cockroachdb: input.is_cockroach(),
        has_migration_table: input.schema.table_walkers().any(is_old_migration_table),
        has_relay_table: input.schema.table_walkers().any(is_relay_table),
        has_prisma_1_join_table: input.schema.table_walkers().any(is_prisma_1_point_0_join_table),
        has_prisma_1_1_or_2_join_table: input.schema.table_walkers().any(is_prisma_1_point_1_or_2_join_table),
        uses_on_delete: false,
        uses_default_values: false,
        always_has_created_at_updated_at: true,
        always_has_p1_or_p_1_1_compatible_id: true,
        uses_non_prisma_types: false,
        has_inline_relations: false,
    };

    for table in input
        .schema
        .table_walkers()
        .filter(|table| !is_old_migration_table(*table))
        .filter(|table| !is_new_migration_table(*table))
        .filter(|table| !is_prisma_1_point_1_or_2_join_table(*table))
        .filter(|table| !is_prisma_1_point_0_join_table(*table))
        .filter(|table| !is_relay_table(*table))
    {
        version_checker.always_has_created_at_updated_at(table);
        version_checker.has_p1_compatible_primary_key_column(table);

        for column in table.columns() {
            version_checker.check_column_for_type_and_default_value(column);
        }

        let foreign_keys = table.foreign_keys();

        if foreign_keys.len() > 0 {
            version_checker.has_inline_relations(table);
        }

        for foreign_key in foreign_keys {
            version_checker.uses_on_delete(foreign_key);
        }
    }

    match version_checker.sql_family {
        _ if input.schema.is_empty() => Version::NonPrisma,
        SqlFamily::Sqlite if version_checker.is_prisma_2() => Version::Prisma2,
        SqlFamily::Sqlite => Version::NonPrisma,
        SqlFamily::Mysql if version_checker.is_prisma_2() => Version::Prisma2,
        SqlFamily::Mysql if version_checker.is_prisma_1() => Version::Prisma1,
        SqlFamily::Mysql if version_checker.is_prisma_1_1() => Version::Prisma11,
        SqlFamily::Mysql => Version::NonPrisma,
        SqlFamily::Postgres if version_checker.is_prisma_2() => Version::Prisma2,
        SqlFamily::Postgres if version_checker.is_prisma_1() => Version::Prisma1,
        SqlFamily::Postgres if version_checker.is_prisma_1_1() => Version::Prisma11,
        SqlFamily::Postgres => Version::NonPrisma,
        SqlFamily::Mssql => Version::NonPrisma,
    }
}

impl VersionChecker {
    /// Check if using only types supported by a legacy version of Prisma.
    fn check_column_for_type_and_default_value(&mut self, column: ColumnWalker<'_>) {
        match self.sql_family {
            SqlFamily::Postgres if self.is_cockroachdb => {
                self.uses_non_prisma_types = true; // we can be sure it's not prisma 1
            }
            SqlFamily::Postgres => {
                if let Some(native_type) = &column.column_type().native_type {
                    let native_type: &PostgresType = native_type.downcast_ref();

                    if !POSTGRES_TYPES.contains(native_type) {
                        self.uses_non_prisma_types = true
                    }
                }
            }
            SqlFamily::Mysql => {
                if let Some(native_type) = &column.column_type().native_type {
                    let native_type: &MySqlType = native_type.downcast_ref();

                    if !MYSQL_TYPES.contains(native_type) {
                        self.uses_non_prisma_types = true
                    }
                }
            }
            SqlFamily::Sqlite if !SQLITE_TYPES.contains(&&*column.column_type().full_data_type) => {
                self.uses_non_prisma_types = true
            }
            _ => (),
        }

        if !column.is_autoincrement() && column.default().is_some() {
            self.uses_default_values = true;
        };
    }

    /// If relations are created with list tables.
    fn has_inline_relations(&mut self, table: TableWalker<'_>) {
        if !is_prisma_1_or_11_list_table(table) {
            self.has_inline_relations = true;
        }
    }

    /// If the database has an explicit `onDelete` column.
    #[allow(clippy::nonminimal_bool)] // more readable this way
    fn uses_on_delete(&mut self, fk: ForeignKeyWalker<'_>) {
        let action = fk.on_delete_action();
        if !(action == ForeignKeyAction::NoAction || action == ForeignKeyAction::SetNull)
            && !is_prisma_1_or_11_list_table(fk.table())
            && action != ForeignKeyAction::Cascade
        {
            self.uses_on_delete = true
        }
    }

    /// If the table always has `createdAt` and `updatedAt` columns.
    fn always_has_created_at_updated_at(&mut self, table: TableWalker<'_>) {
        if !is_prisma_1_or_11_list_table(table) && !is_relay_table(table) && !has_created_at_and_updated_at(table) {
            self.always_has_created_at_updated_at = false
        }
    }

    /// If the primary key is defined in a way how Prisma 1 defined
    /// them.
    fn has_p1_compatible_primary_key_column(&mut self, table: TableWalker<'_>) {
        if self.is_cockroachdb {
            // we rule out crdb + P1
            return;
        }

        if !is_prisma_1_or_11_list_table(table) && !is_relay_table(table) {
            if let Some(pk) = table.primary_key() {
                if pk.columns().len() == 1 {
                    let col = pk.columns().next().unwrap();
                    let tpe = col.as_column().column_type();

                    if self.sql_family == SqlFamily::Postgres {
                        if let Some(native_type) = &tpe.native_type {
                            let native_type: &PostgresType = native_type.downcast_ref();

                            if native_type != &PostgresType::VarChar(Some(25))
                                && native_type != &PostgresType::VarChar(Some(36))
                                && native_type != &PostgresType::Integer
                            {
                                self.always_has_p1_or_p_1_1_compatible_id = false
                            }
                        }
                    } else if self.sql_family == SqlFamily::Mysql {
                        if let Some(native_type) = &tpe.native_type {
                            let native_type: &MySqlType = native_type.downcast_ref();

                            if native_type != &MySqlType::Char(25)
                                && native_type != &MySqlType::Char(36)
                                && native_type != &MySqlType::Int
                            {
                                self.always_has_p1_or_p_1_1_compatible_id = false
                            }
                        }
                    };
                }
            }
        }
    }

    /// If we're running a "modern" Prisma.
    fn is_prisma_2(&self) -> bool {
        !self.has_relay_table && !self.uses_on_delete && !self.uses_non_prisma_types && self.has_migration_table
    }

    /// If the database is from Prisma 1.1
    fn is_prisma_1_1(&self) -> bool {
        !self.has_migration_table
            && !self.has_relay_table
            && !self.uses_on_delete
            && !self.uses_default_values
            && !self.uses_non_prisma_types
            && !self.has_prisma_1_join_table
            && self.always_has_p1_or_p_1_1_compatible_id
    }

    /// If the database is from Prisma 1.0
    fn is_prisma_1(&self) -> bool {
        !self.has_migration_table
            && !self.uses_on_delete
            && !self.uses_default_values
            && !self.uses_non_prisma_types
            && !self.has_prisma_1_1_or_2_join_table
            && !self.has_inline_relations
            && self.has_relay_table
            && self.always_has_created_at_updated_at
            && self.always_has_p1_or_p_1_1_compatible_id
    }
}
