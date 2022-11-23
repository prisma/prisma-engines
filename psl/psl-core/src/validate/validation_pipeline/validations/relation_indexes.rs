use crate::{datamodel_connector::RelationMode, validate::validation_pipeline::context::Context};
use diagnostics::DatamodelWarning;
use parser_database::ast::WithSpan;
use parser_database::{
    ast::ModelId,
    walkers::{RelationFieldWalker, Walker},
};

// { x_1, x_2, ..., x_n } is left-wise included in { y_1, y_2, ..., y_m } if and only if
// n <= m and x_i = y_i for all i in [1, n].
fn is_leftwise_included_it<T>(item: impl ExactSizeIterator<Item = T>, group: impl Iterator<Item = T>) -> bool
where
    T: PartialEq,
{
    group.take(item.len()).eq(item)
}

pub(super) fn validate_missing_relation_indexes(
    model: Walker<'_, ModelId>,
    relation_field: RelationFieldWalker<'_>,
    ctx: &mut Context<'_>,
) {
    if !ctx.connector.should_suggest_missing_referencing_fields_indexes() || ctx.relation_mode != RelationMode::Prisma {
        return;
    }

    if let Some(fields) = relation_field.referencing_fields() {
        // Considers all fields that should be part of an index in the given model, w.r.t. to left-wise inclusion.
        let referencing_fields_it = fields.map(|field| field.field_id());

        // Considers all groups of indexes explicitly declared in the given model.
        // An index group can be:
        // - a singleton (@unique or @id)
        // - an ordered set (@@unique or @@index)
        let index_field_groups = model.indexes();

        let referencing_fields_appear_in_index = index_field_groups
            .map(|index_walker| index_walker.fields().map(|index| index.field_id()))
            .any(|index_fields_it| {
                let fields_it = referencing_fields_it.clone();
                is_leftwise_included_it(fields_it, index_fields_it)
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

#[cfg(test)]
mod tests {
    use super::is_leftwise_included_it;
    #[test]
    fn test_is_left_wise_included() {
        let item = vec![1, 2];
        let group = vec![1, 2, 3, 4];
        assert_eq!(is_leftwise_included_it(item.iter(), group.iter()), true);
        let item = vec![1, 2, 3, 4];
        let group = vec![1, 2, 3, 4];
        assert_eq!(is_leftwise_included_it(item.iter(), group.iter()), true);
        let item = vec![1, 2, 3, 4];
        let group = vec![1, 2];
        assert_eq!(is_leftwise_included_it(item.iter(), group.iter()), false);
        let item = vec![2, 3];
        let group = vec![1, 2, 3, 4];
        assert_eq!(is_leftwise_included_it(item.iter(), group.iter()), false);
    }
}
