use sql::{postgres::PostgresSchemaExt, ForeignKeyWalker, IndexWalker, TableWalker};
use sql_schema_describer as sql;

use crate::{
    datamodel_calculator::DatamodelCalculatorContext,
    warnings::generators::{CheckConstraint, ExclusionConstraint, IndexedColumn, Model, ModelAndConstraint, Warnings},
};

pub(crate) struct PostgresIntrospectionFlavour;

impl super::IntrospectionFlavour for PostgresIntrospectionFlavour {
    fn keep_previous_scalar_field_arity(&self, next: sql::ColumnWalker<'_>) -> bool {
        next.is_in_view() && next.column_type().arity.is_nullable()
    }

    fn generate_warnings(&self, ctx: &DatamodelCalculatorContext<'_>, warnings: &mut Warnings) {
        for table in ctx.sql_schema.table_walkers() {
            for index in table.indexes() {
                for column in index.columns().filter(|c| self.uses_non_default_null_position(ctx, *c)) {
                    warnings.non_default_index_null_sort_order.push(IndexedColumn {
                        index_name: index.name().to_string(),
                        column_name: column.name().to_string(),
                    });
                }

                if self.uses_non_default_index_deferring(ctx, index) {
                    warnings.non_default_deferring.push(ModelAndConstraint {
                        model: ctx.table_prisma_name(table.id).prisma_name().to_string(),
                        constraint: index.name().to_string(),
                    });
                }
            }

            for fk in table.foreign_keys() {
                if self.uses_non_default_foreign_key_deferring(ctx, fk) {
                    warnings.non_default_deferring.push(ModelAndConstraint {
                        model: ctx.table_prisma_name(table.id).prisma_name().to_string(),
                        // unwrap: postgres fks always have a name
                        constraint: fk.constraint_name().unwrap().to_string(),
                    });
                }
            }

            if self.uses_row_level_ttl(ctx, table) {
                warnings.row_level_ttl.push(Model {
                    model: ctx.table_prisma_name(table.id).prisma_name().to_string(),
                });
            }
        }

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

    fn uses_row_level_ttl(&self, ctx: &DatamodelCalculatorContext<'_>, table: TableWalker<'_>) -> bool {
        let pg_ext: &PostgresSchemaExt = ctx.sql_schema.downcast_connector_data();

        ctx.is_cockroach() && pg_ext.uses_row_level_ttl(table.id)
    }

    fn uses_non_default_index_deferring(&self, ctx: &DatamodelCalculatorContext<'_>, index: IndexWalker<'_>) -> bool {
        let pg_ext: &PostgresSchemaExt = ctx.sql_schema.downcast_connector_data();

        pg_ext.non_default_index_constraint_deferring(index.id)
    }

    fn uses_non_default_foreign_key_deferring(
        &self,
        ctx: &DatamodelCalculatorContext<'_>,
        foreign_key: ForeignKeyWalker<'_>,
    ) -> bool {
        let pg_ext: &PostgresSchemaExt = ctx.sql_schema.downcast_connector_data();

        pg_ext.non_default_foreign_key_constraint_deferring(foreign_key.id)
    }

    fn uses_non_default_null_position(
        &self,
        ctx: &DatamodelCalculatorContext<'_>,
        column: sql::IndexColumnWalker<'_>,
    ) -> bool {
        let pg_ext: &PostgresSchemaExt = ctx.sql_schema.downcast_connector_data();

        pg_ext.non_default_null_position(column)
    }
}
