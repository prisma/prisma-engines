use crate::misc_helpers::{
    is_migration_table, is_prisma_1_or_11_list_table, is_prisma_1_point_0_join_table,
    is_prisma_1_point_1_or_2_join_table, is_relay_table,
};
use datamodel::Model;
use introspection_connector::{Version, Warning};
use quaint::connector::SqlFamily;
use sql_schema_describer::{ColumnType, ForeignKey, ForeignKeyAction, SqlSchema, Table};

pub struct VersionChecker {
    sql_family: SqlFamily,
    migration_table: bool,
    has_prisma_1_join_table: bool,
    has_prisma_1_1_or_2_join_table: bool,
    uses_on_delete: bool,
    always_has_created_at_updated_at: bool,
    uses_non_prisma_types: bool,
    has_inline_relations: bool,
}

const SQLITE_TYPES: [(&'static str, &'static str); 5] = [
    ("BOOLEAN", "BOOLEAN"),
    ("DATE", "DATE"),
    ("REAL", "REAL"),
    ("INTEGER", "INTEGER"),
    ("TEXT", "TEXT"),
];

const POSTGRES_TYPES: [(&'static str, &'static str); 5] = [
    ("boolean", "bool"),
    ("timestamp without time zone", "timestamp"),
    ("numeric", "numeric"),
    ("integer", "int4"),
    ("text", "text"),
];
const MYSQL_TYPES: [(&'static str, &'static str); 13] = [
    ("tinyint", "tinyint(1)"),
    ("datetime", "datetime(3)"),
    ("decimal", "decimal(65,30)"),
    ("int", "int"),
    ("int", "int(11)"),
    ("varchar", "varchar(191)"),
    ("char", "char(25)"),
    ("char", "char(36)"),
    ("varchar", "varchar(25)"),
    ("varchar", "varchar(36)"),
    ("text", "text"),
    ("mediumtext", "mediumtext"),
    ("int", "int(4)"),
];

impl VersionChecker {
    pub fn new(sql_family: SqlFamily, schema: &SqlSchema) -> VersionChecker {
        VersionChecker {
            sql_family,
            migration_table: schema.tables.iter().any(|table| is_migration_table(&table)),
            has_prisma_1_join_table: schema.tables.iter().any(|table| is_prisma_1_point_0_join_table(&table)),
            has_prisma_1_1_or_2_join_table: schema
                .tables
                .iter()
                .any(|table| is_prisma_1_point_1_or_2_join_table(&table)),
            uses_on_delete: false,
            always_has_created_at_updated_at: true,
            uses_non_prisma_types: false,
            has_inline_relations: false,
        }
    }

    pub fn uses_non_prisma_type(&mut self, tpe: &ColumnType) {
        match (&tpe.data_type, &tpe.full_data_type, self.sql_family) {
            (dt, fdt, SqlFamily::Postgres) if !POSTGRES_TYPES.contains(&(dt, fdt)) => self.uses_non_prisma_types = true,
            (dt, fdt, SqlFamily::Mysql) if !MYSQL_TYPES.contains(&(dt, fdt)) => self.uses_non_prisma_types = true,
            (dt, fdt, SqlFamily::Sqlite) if !SQLITE_TYPES.contains(&(dt, fdt)) => self.uses_non_prisma_types = true,
            _ => (),
        };
    }

    pub fn has_inline_relations(&mut self, table: &Table) {
        if !is_prisma_1_or_11_list_table(table) {
            self.has_inline_relations = true;
        }
    }

    pub fn uses_on_delete(&mut self, fk: &ForeignKey, table: &Table) {
        if !(fk.on_delete_action == ForeignKeyAction::NoAction || fk.on_delete_action == ForeignKeyAction::SetNull) {
            if !is_prisma_1_or_11_list_table(table) && fk.on_delete_action != ForeignKeyAction::Cascade {
                self.uses_on_delete = true
            }
        }
    }

    pub fn always_has_created_at_updated_at(&mut self, table: &Table, model: &Model) {
        if !is_prisma_1_or_11_list_table(table) && !is_relay_table(table) && !model.has_created_at_and_updated_at() {
            self.always_has_created_at_updated_at = false
        }
    }

    pub fn version(&self, warnings: &Vec<Warning>) -> Version {
        match self.sql_family {
            SqlFamily::Sqlite
                if self.migration_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && warnings.is_empty() =>
            {
                Version::Prisma2
            }
            SqlFamily::Sqlite => Version::NonPrisma,
            SqlFamily::Mysql
                if self.migration_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && warnings.is_empty() =>
            {
                Version::Prisma2
            }
            SqlFamily::Mysql
                if !self.migration_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && self.always_has_created_at_updated_at
                    && !self.has_prisma_1_1_or_2_join_table
                    && !self.has_inline_relations
                    && warnings.is_empty() =>
            {
                Version::Prisma1
            }
            SqlFamily::Mysql
                if !self.migration_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && !self.has_prisma_1_join_table
                    && warnings.is_empty() =>
            {
                Version::Prisma11
            }
            SqlFamily::Mysql => Version::NonPrisma,
            SqlFamily::Postgres
                if self.migration_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && warnings.is_empty() =>
            {
                Version::Prisma2
            }
            SqlFamily::Postgres
                if !self.migration_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && self.always_has_created_at_updated_at
                    && !self.has_prisma_1_join_table
                    && !self.has_inline_relations
                    && warnings.is_empty() =>
            {
                Version::Prisma1
            }
            SqlFamily::Postgres
                if !self.migration_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && !self.has_prisma_1_1_or_2_join_table
                    && warnings.is_empty() =>
            {
                Version::Prisma11
            }
            SqlFamily::Postgres => Version::NonPrisma,
        }
    }
}
