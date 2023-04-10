use sql::postgres::PostgresSchemaExt;
use sql_schema_describer as sql;

use crate::{
    datamodel_calculator::DatamodelCalculatorContext,
    warnings::generators::{IndexedColumn, Warnings},
};

pub(crate) struct PostgresIntrospectionFlavour;

impl super::IntrospectionFlavour for PostgresIntrospectionFlavour {
    fn keep_previous_scalar_field_arity(&self, next: sql::ColumnWalker<'_>) -> bool {
        next.is_in_view() && next.column_type().arity.is_nullable()
    }

    fn generate_warnings(&self, ctx: &DatamodelCalculatorContext<'_>, warnings: &mut Warnings) {
        let pg_ext: &PostgresSchemaExt = ctx.sql_schema.downcast_connector_data();

        for index in ctx.sql_schema.table_walkers().flat_map(|t| t.indexes()) {
            for column in index.columns().filter(|c| pg_ext.non_default_null_position(*c)) {
                warnings.non_default_index_null_sort_order.push(IndexedColumn {
                    index_name: index.name().to_string(),
                    column_name: column.name().to_string(),
                });
            }
        }
    }
}
