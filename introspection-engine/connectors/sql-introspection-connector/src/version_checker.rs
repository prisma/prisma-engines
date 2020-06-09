use crate::misc_helpers::{
    is_migration_table, is_prisma_1_or_11_list_table, is_prisma_1_point_0_join_table,
    is_prisma_1_point_1_or_2_join_table, is_relay_table,
};
use datamodel::Model;
use introspection_connector::{Version, Warning};
use quaint::connector::SqlFamily;
use sql_schema_describer::{ColumnType, ForeignKey, ForeignKeyAction, PrimaryKey, SqlSchema, Table};

pub struct VersionChecker {
    sql_family: SqlFamily,
    has_migration_table: bool,
    has_relay_table: bool,
    has_prisma_1_join_table: bool,
    has_prisma_1_1_or_2_join_table: bool,
    uses_on_delete: bool,
    always_has_created_at_updated_at: bool,
    always_has_p1_compatible_id: bool,
    uses_non_prisma_types: bool,
    has_inline_relations: bool,
}

// todo More ideas for possible tightening
// P1/P11 were not using db level default values

const CHAR: &str = "char";
const CHAR_25: &str = "char(25)";
const CHAR_36: &str = "char(36)";
const INT: &str = "int";
const INT_11: &str = "int(11)";
const INTEGER: &str = "integer";
const INT_4: &str = "int4";
const VARCHAR: &str = "varchar";
const CHARACTER_VARYING: &str = "character varying";

const SQLITE_TYPES: &'static [(&'static str, &'static str)] = &[
    ("BOOLEAN", "BOOLEAN"),
    ("DATE", "DATE"),
    ("REAL", "REAL"),
    ("INTEGER", "INTEGER"),
    ("TEXT", "TEXT"),
];

const POSTGRES_TYPES: &'static [(&'static str, &'static str)] = &[
    ("boolean", "bool"),
    ("timestamp without time zone", "timestamp"),
    ("numeric", "numeric"),
    ("integer", "int4"),
    ("text", "text"),
    ("character varying", "varchar"),
];
const MYSQL_TYPES: &'static [(&'static str, &'static str)] = &[
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
            has_migration_table: schema.tables.iter().any(|table| is_migration_table(&table)),
            has_relay_table: schema.tables.iter().any(|table| is_relay_table(&table)),
            has_prisma_1_join_table: schema.tables.iter().any(|table| is_prisma_1_point_0_join_table(&table)),
            has_prisma_1_1_or_2_join_table: schema
                .tables
                .iter()
                .any(|table| is_prisma_1_point_1_or_2_join_table(&table)),
            uses_on_delete: false,
            always_has_created_at_updated_at: true,
            always_has_p1_compatible_id: true,
            uses_non_prisma_types: false,
            has_inline_relations: false,
        }
    }

    //todo Possible further tightening
    //P1/P11 only limited strings to specific lengths on ids
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

    pub fn has_p1_compatible_primary_key_column(&mut self, table: &Table) {
        if !is_prisma_1_or_11_list_table(table) && !is_relay_table(table) {
            if let Some(PrimaryKey { columns, .. }) = &table.primary_key {
                if columns.len() == 1 {
                    let tpe = &table.column_bang(columns.first().unwrap()).tpe;

                    match (
                        &tpe.data_type,
                        &tpe.full_data_type,
                        &tpe.character_maximum_length,
                        self.sql_family,
                    ) {
                        (dt, fdt, Some(25), SqlFamily::Mysql) if dt == CHAR && fdt == CHAR_25 => (),
                        (dt, fdt, Some(36), SqlFamily::Mysql) if dt == CHAR && fdt == CHAR_36 => (),
                        (dt, fdt, None, SqlFamily::Mysql) if dt == INT && (fdt == INT_11 || fdt == INT) => (),
                        (dt, fdt, Some(25), SqlFamily::Postgres) if dt == CHARACTER_VARYING && fdt == VARCHAR => (),
                        (dt, fdt, Some(36), SqlFamily::Postgres) if dt == CHARACTER_VARYING && fdt == VARCHAR => (),
                        (dt, fdt, None, SqlFamily::Postgres) if dt == INTEGER && fdt == INT_4 => (),
                        _ => self.always_has_p1_compatible_id = false,
                    }
                }
            }
        }
    }

    pub fn version(&self, warnings: &Vec<Warning>) -> Version {
        match self.sql_family {
            SqlFamily::Sqlite
                if self.has_migration_table
                    && !self.has_relay_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && warnings.is_empty() =>
            {
                Version::Prisma2
            }
            SqlFamily::Sqlite => Version::NonPrisma,
            SqlFamily::Mysql
                if self.has_migration_table
                    && !self.has_relay_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && warnings.is_empty() =>
            {
                Version::Prisma2
            }
            SqlFamily::Mysql
                if !self.has_migration_table
                    && self.has_relay_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && self.always_has_created_at_updated_at
                    && self.always_has_p1_compatible_id
                    && !self.has_prisma_1_1_or_2_join_table
                    && !self.has_inline_relations
                    && warnings.is_empty() =>
            {
                Version::Prisma1
            }
            SqlFamily::Mysql
                if !self.has_migration_table
                    && !self.has_relay_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && self.always_has_p1_compatible_id
                    && !self.has_prisma_1_join_table
                    && warnings.is_empty() =>
            {
                Version::Prisma11
            }
            SqlFamily::Mysql => Version::NonPrisma,
            SqlFamily::Postgres
                if self.has_migration_table
                    && !self.has_relay_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && warnings.is_empty() =>
            {
                Version::Prisma2
            }
            SqlFamily::Postgres
                if !self.has_migration_table
                    && self.has_relay_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && self.always_has_created_at_updated_at
                    && !self.has_prisma_1_1_or_2_join_table
                    && !self.has_inline_relations
                    && self.always_has_p1_compatible_id
                    && warnings.is_empty() =>
            {
                Version::Prisma1
            }
            SqlFamily::Postgres
                if !self.has_migration_table
                    && !self.has_relay_table
                    && !self.uses_on_delete
                    && !self.uses_non_prisma_types
                    && !self.has_prisma_1_join_table
                    && self.always_has_p1_compatible_id
                    && warnings.is_empty() =>
            {
                Version::Prisma11
            }
            SqlFamily::Postgres => Version::NonPrisma,
        }
    }
}
