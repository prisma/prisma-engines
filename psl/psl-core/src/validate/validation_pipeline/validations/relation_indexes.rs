use diagnostics::DatamodelWarning;
use parser_database::{
    ast::{FieldId, ModelId},
    walkers::{RelationFieldWalker, Walker},
};

use crate::{datamodel_connector::RelationMode, validate::validation_pipeline::context::Context};

// { x_1, x_2, ..., x_n } is left-wise included in { y_1, y_2, ..., y_m } if and only if
// n <= m and x_i = y_i for all i in [1, n].
fn is_left_wise_included<T>(item: &Vec<T>, group: &Vec<T>) -> bool
where
    T: PartialEq,
{
    group.iter().take(item.len()).eq(item.iter())
}

pub(super) fn validate_missing_relation_indexes(
    model: Walker<'_, ModelId>,
    relation_field: RelationFieldWalker<'_>,
    ctx: &mut Context<'_>,
) {
    dbg!("Inspecting: {:?}", relation_field.name());

    let is_provider_mongodb = ctx
        .datasource
        .map(|datasource| datasource.active_provider == "mongodb")
        .unwrap_or(false);

    if is_provider_mongodb || ctx.relation_mode != RelationMode::Prisma {
        return;
    }

    if let Some(fields) = relation_field.referenced_fields() {
        // Collects all fields that should be part of an index in the given model, w.r.t. to left-wise inclusion.
        let relation_fields: Vec<FieldId> = fields.map(|field| field.field_id()).collect();

        // Collects all groups of indexes explicitly declared in the given model.
        // An index group can be:
        // - a singleton (@unique or @id)
        // - an ordered set (@@unique or @@index)
        let index_sets: Vec<Vec<FieldId>> = model
            .indexes()
            .map(|index_walker| index_walker.fields().map(|index| index.field_id()).collect())
            .collect();

        let relation_fields_appear_in_index = index_sets
            .iter()
            .any(|index_set| is_left_wise_included(&relation_fields, index_set));

        if !(relation_fields_appear_in_index) {
            let span = relation_field.ast_field().field_type.span();
            ctx.push_warning(DatamodelWarning::new_missing_index_on_emulated_relation(span));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_left_wise_included;

    #[test]
    fn test_is_left_wise_included() {
        let item = vec![1, 2];
        let group = vec![1, 2, 3, 4];
        assert_eq!(is_left_wise_included(&item, &group), true);

        let item = vec![1, 2, 3, 4];
        let group = vec![1, 2];
        assert_eq!(is_left_wise_included(&item, &group), false);

        let item = vec![2, 3];
        let group = vec![1, 2, 3, 4];
        assert_eq!(is_left_wise_included(&item, &group), false);
    }
}
