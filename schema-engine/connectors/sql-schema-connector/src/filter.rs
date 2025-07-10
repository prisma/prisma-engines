use sql_schema_describer::SqlSchema;

use crate::{database_schema::SqlDatabaseSchema, SchemaFilter};

pub fn filter_sql_database_schema(schema: SqlDatabaseSchema, filter: SchemaFilter) -> SqlDatabaseSchema {
    SqlDatabaseSchema {
        describer_schema: filter_sql_schema(schema.describer_schema, filter),
        prisma_level_defaults: schema.prisma_level_defaults,
    }
}

pub fn filter_sql_schema(schema: SqlSchema, _filter: SchemaFilter) -> SqlSchema {
    // TODO: Implement filtering logic
    schema
}
