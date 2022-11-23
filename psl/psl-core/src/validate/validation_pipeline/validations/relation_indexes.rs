use diagnostics::DatamodelWarning;
use parser_database::ast::WithSpan;
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
    if !ctx.connector.should_suggest_missing_referencing_fields_indexes() || ctx.relation_mode != RelationMode::Prisma {
        return;
    }

    dbg!("Check wasn't skipped");

    if let Some(fields) = relation_field.referencing_fields() {
        // Collects all fields that should be part of an index in the given model, w.r.t. to left-wise inclusion.
        let referencing_fields: Vec<_> = fields.map(|field| field.field_id()).collect();

        // Considers all groups of indexes explicitly declared in the given model.
        // An index group can be:
        // - a singleton (@unique or @id)
        // - an ordered set (@@unique or @@index)
        let referencing_fields_appear_in_index = model
            .indexes()
            .map(|index_walker| index_walker.fields().map(|index| index.field_id()))
            .any(|index_fields_it| {
                // { x_1, x_2, ..., x_n } is left-wise included in { y_1, y_2, ..., y_m } if and only if
                // n <= m and x_i = y_i for all i in [1, n].
                let is_leftwise_included_new = referencing_fields.len() <= index_fields_it.len()
                    && referencing_fields
                        .iter()
                        .zip(index_fields_it)
                        .all(|(referencing_field, index_field)| *referencing_field == index_field);

                is_leftwise_included_new
            });

        if !referencing_fields_appear_in_index {
            let ast_field = relation_field.ast_field();
            let span = ast_field
                .span_for_attribute("relation")
                .unwrap_or_else(|| ast_field.span());
            ctx.push_warning(DatamodelWarning::new_missing_index_on_emulated_relation(span));
        }
    }
}
