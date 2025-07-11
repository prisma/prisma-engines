use sql_schema_describer::{filter::SqlSchemaFilter, SqlSchema};

use crate::{database_schema::SqlDatabaseSchema, SchemaFilter};

pub fn filter_sql_database_schema(schema: SqlDatabaseSchema, filter: &SchemaFilter) -> SqlDatabaseSchema {
    SqlDatabaseSchema {
        describer_schema: filter_sql_schema(schema.describer_schema, filter),
        prisma_level_defaults: schema.prisma_level_defaults,
    }
}

pub fn filter_sql_schema(schema: SqlSchema, _filter: &SchemaFilter) -> SqlSchema {
    // TODO:(schema-filter) temporarily a noop while testing that the other approach with filtering during diffing works
    schema
    // let sql_filter = SqlSchemaFilter {
    //     external_tables: filter.external_tables.clone(),
    // };
    // schema.filter(&sql_filter)
}
