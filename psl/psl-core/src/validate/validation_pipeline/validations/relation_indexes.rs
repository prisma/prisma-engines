use diagnostics::DatamodelWarning;
use parser_database::{
    ast::ModelId,
    walkers::{RelationFieldWalker, Walker},
};

use crate::{datamodel_connector::RelationMode, validate::validation_pipeline::context::Context};

pub(super) fn validate_missing_relation_indexes(
    model: Walker<'_, ModelId>,
    relation_field: RelationFieldWalker<'_>,
    ctx: &mut Context<'_>,
) {
    if ctx.connector.should_suggest_missing_referencing_fields_indexes() || ctx.relation_mode != RelationMode::Prisma {
        return;
    }

    if let Some(fields) = relation_field.referencing_fields() {
        let referencing_fields: Vec<_> = fields.collect();

        let referencing_fields_appear_in_at_least_one_index = model.indexes().any(|index_walker| {
            // Considers all groups of indexes explicitly declared in the given model.
            // An index group can be:
            // - a singleton (@unique or @id)
            // - an ordered set (@@unique or @@index)
            let index_fields = index_walker.fields();

            let fields = &referencing_fields;

            // { x_1, x_2, ..., x_n } is left-wise included in { y_1, y_2, ..., y_m } if and only if
            // n <= m and x_i = y_i for all i in [1, n].
            let are_referencing_fields_leftwise_included = fields.len() <= index_fields.len()
                && fields
                    .iter()
                    .zip(index_fields)
                    .all(|(a, b)| a.field_id() == b.field_id());
            are_referencing_fields_leftwise_included
        });

        if !referencing_fields_appear_in_at_least_one_index {
            let span = relation_field.ast_field().field_type.span();
            ctx.push_warning(DatamodelWarning::new_missing_index_on_emulated_relation(span));
        }
    }
}
