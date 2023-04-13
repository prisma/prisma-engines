use sql::postgres::PostgresSchemaExt;
use sql_schema_describer as sql;

use crate::{
    datamodel_calculator::DatamodelCalculatorContext,
    warnings::generators::{IndexedColumn, Model, Warnings},
};

pub(crate) struct PostgresIntrospectionFlavour;

impl super::IntrospectionFlavour for PostgresIntrospectionFlavour {
    fn keep_previous_scalar_field_arity(&self, next: sql::ColumnWalker<'_>) -> bool {
        next.is_in_view() && next.column_type().arity.is_nullable()
    }

    fn generate_warnings(&self, ctx: &DatamodelCalculatorContext<'_>, warnings: &mut Warnings) {
        let pg_ext: &PostgresSchemaExt = ctx.sql_schema.downcast_connector_data();

        for table in ctx.sql_schema.table_walkers() {
            for index in table.indexes() {
                for column in index.columns().filter(|c| pg_ext.non_default_null_position(*c)) {
                    warnings.non_default_index_null_sort_order.push(IndexedColumn {
                        index_name: index.name().to_string(),
                        column_name: column.name().to_string(),
                    });
                }
            }

            if pg_ext.uses_row_level_ttl(table.id) && ctx.is_cockroach() {
                warnings.row_level_ttl.push(Model {
                    model: ctx.table_prisma_name(table.id).prisma_name().to_string(),
                })
            }
        }
    }
}
