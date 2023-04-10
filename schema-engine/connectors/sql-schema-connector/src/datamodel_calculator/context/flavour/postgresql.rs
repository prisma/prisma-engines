use sql::postgres::PostgresSchemaExt;
use sql_schema_describer as sql;

use crate::{
    datamodel_calculator::DatamodelCalculatorContext,
    warnings::generators::{CheckConstraint, ExclusionConstraint, IndexedColumn, Warnings},
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

        dbg!("check_constraints: {}", &ctx.sql_schema.check_constraints());

        for check_constraint in ctx.sql_schema.check_constraints() {
            let check_constraint = CheckConstraint {
                name: check_constraint.name.clone(),
                definition: check_constraint.definition.clone(),
            };
            warnings.check_constraints.push(check_constraint);
        }

        for exclusion_constraint in ctx.sql_schema.exclusion_constraints() {
            let exclusion_constraint = ExclusionConstraint {
                name: exclusion_constraint.name.clone(),
                definition: exclusion_constraint.definition.clone(),
            };
            warnings.exclusion_constraints.push(exclusion_constraint);
        }
    }
}
