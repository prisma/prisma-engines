use crate::introspection_helpers::{
    is_old_migration_table, is_prisma_1_or_11_list_table, is_prisma_1_point_0_join_table,
    is_prisma_1_point_1_or_2_join_table, is_relay_table,
};
use crate::SqlFamilyTrait;
use datamodel::{Datamodel, Model};
use introspection_connector::{IntrospectionContext, Version, Warning};
use native_types::{MySqlType, PostgresType};
use quaint::connector::SqlFamily;
use sql_schema_describer::{Column, ForeignKey, ForeignKeyAction, PrimaryKey, SqlSchema, Table};
use tracing::debug;

#[derive(Debug)]
pub struct VersionChecker {
    sql_family: SqlFamily,
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

const SQLITE_TYPES: &[&str] = &["BOOLEAN", "DATE", "REAL", "INTEGER", "TEXT"];

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

impl VersionChecker {
    pub fn new(schema: &SqlSchema, ctx: &IntrospectionContext) -> VersionChecker {
        VersionChecker {
            sql_family: ctx.sql_family(),
            has_migration_table: schema.tables.iter().any(|table| is_old_migration_table(table)),
            has_relay_table: schema.tables.iter().any(|table| is_relay_table(table)),
            has_prisma_1_join_table: schema.tables.iter().any(|table| is_prisma_1_point_0_join_table(table)),
            has_prisma_1_1_or_2_join_table: schema
                .tables
                .iter()
                .any(|table| is_prisma_1_point_1_or_2_join_table(table)),
            uses_on_delete: false,
            uses_default_values: false,
            always_has_created_at_updated_at: true,
            always_has_p1_or_p_1_1_compatible_id: true,
            uses_non_prisma_types: false,
            has_inline_relations: false,
        }
    }

    pub fn check_column_for_type_and_default_value(&mut self, column: &Column) {
        match self.sql_family {
            SqlFamily::Postgres => {
                if let Some(native_type) = &column.tpe.native_type {
                    let native_type: PostgresType = serde_json::from_value(native_type.clone()).unwrap();

                    if !POSTGRES_TYPES.contains(&native_type) {
                        self.uses_non_prisma_types = true
                    }
                }
            }
            SqlFamily::Mysql => {
                if let Some(native_type) = &column.tpe.native_type {
                    let native_type: MySqlType = serde_json::from_value(native_type.clone()).unwrap();

                    if !MYSQL_TYPES.contains(&native_type) {
                        self.uses_non_prisma_types = true
                    }
                }
            }
            SqlFamily::Sqlite if !SQLITE_TYPES.contains(&&*column.tpe.full_data_type) => {
                self.uses_non_prisma_types = true
            }
            _ => (),
        }

        if !column.auto_increment && column.default.is_some() {
            self.uses_default_values = true;
        };
    }

    pub fn has_inline_relations(&mut self, table: &Table) {
        if !is_prisma_1_or_11_list_table(table) {
            self.has_inline_relations = true;
        }
    }

    pub fn uses_on_delete(&mut self, fk: &ForeignKey, table: &Table) {
        if !(fk.on_delete_action == ForeignKeyAction::NoAction || fk.on_delete_action == ForeignKeyAction::SetNull)
            && !is_prisma_1_or_11_list_table(table)
            && fk.on_delete_action != ForeignKeyAction::Cascade
        {
            self.uses_on_delete = true
        }
    }

    pub fn always_has_created_at_updated_at(&mut self, table: &Table, model: &Model) {
        if !is_prisma_1_or_11_list_table(table) && !is_relay_table(table) && !model.has_created_at_and_updated_at() {
            self.always_has_created_at_updated_at = false
        }
    }

    pub fn has_p1_compatible_primary_key_column(&mut self, table: &Table) {
        if !is_prisma_1_or_11_list_table(table) && !is_relay_table(table) {
            if let Some(PrimaryKey { columns, .. }) = &table.primary_key {
                if columns.len() == 1 {
                    let tpe = &table.column_bang(columns.first().unwrap()).tpe;

                    if self.sql_family == SqlFamily::Postgres {
                        if let Some(native_type) = &tpe.native_type {
                            let native_type: PostgresType = serde_json::from_value(native_type.clone()).unwrap();

                            if native_type != PostgresType::VarChar(Some(25))
                                && native_type != PostgresType::VarChar(Some(36))
                                && native_type != PostgresType::Integer
                            {
                                self.always_has_p1_or_p_1_1_compatible_id = false
                            }
                        }
                    } else if self.sql_family == SqlFamily::Mysql {
                        if let Some(native_type) = &tpe.native_type {
                            let native_type: MySqlType = serde_json::from_value(native_type.clone()).unwrap();

                            if native_type != MySqlType::Char(25)
                                && native_type != MySqlType::Char(36)
                                && native_type != MySqlType::Int
                            {
                                self.always_has_p1_or_p_1_1_compatible_id = false
                            }
                        }
                    };
                }
            }
        }
    }

    fn is_prisma_2(&self, warnings: &[Warning]) -> bool {
        !self.has_relay_table
            && !self.uses_on_delete
            && !self.uses_non_prisma_types
            && self.has_migration_table
            && warnings.is_empty()
    }

    fn is_prisma_1_1(&self, warnings: &[Warning]) -> bool {
        !self.has_migration_table
            && !self.has_relay_table
            && !self.uses_on_delete
            && !self.uses_default_values
            && !self.uses_non_prisma_types
            && !self.has_prisma_1_join_table
            && self.always_has_p1_or_p_1_1_compatible_id
            && warnings.is_empty()
    }

    fn is_prisma_1(&self, warnings: &[Warning]) -> bool {
        !self.has_migration_table
            && !self.uses_on_delete
            && !self.uses_default_values
            && !self.uses_non_prisma_types
            && !self.has_prisma_1_1_or_2_join_table
            && !self.has_inline_relations
            && self.has_relay_table
            && self.always_has_created_at_updated_at
            && self.always_has_p1_or_p_1_1_compatible_id
            && warnings.is_empty()
    }

    pub fn version(&self, warnings: &[Warning], data_model: &Datamodel) -> Version {
        debug!("{:?}", &self);
        match self.sql_family {
            _ if data_model.is_empty() => Version::NonPrisma,
            SqlFamily::Sqlite if self.is_prisma_2(warnings) => Version::Prisma2,
            SqlFamily::Sqlite => Version::NonPrisma,
            SqlFamily::Mysql if self.is_prisma_2(warnings) => Version::Prisma2,
            SqlFamily::Mysql if self.is_prisma_1(warnings) => Version::Prisma1,
            SqlFamily::Mysql if self.is_prisma_1_1(warnings) => Version::Prisma11,
            SqlFamily::Mysql => Version::NonPrisma,
            SqlFamily::Postgres if self.is_prisma_2(warnings) => Version::Prisma2,
            SqlFamily::Postgres if self.is_prisma_1(warnings) => Version::Prisma1,
            SqlFamily::Postgres if self.is_prisma_1_1(warnings) => Version::Prisma11,
            SqlFamily::Postgres => Version::NonPrisma,
            SqlFamily::Mssql => Version::NonPrisma,
        }
    }
}
