use diagnostics::DatamodelWarning;
use parser_database::{
    ast::ModelId,
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
    _model: Walker<'_, ModelId>,
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
        // vector of ids of the fields that should be part of an index in the given model, w.r.t. to left-wise inclusion.
        let relation_fields = fields.map(|field| field.field_id()).collect::<Vec<_>>();

        // for index_walker in model.indexes() {
        //     dbg!("exploring index: {:?}", index_walker.name());
        //     index_walker.fields().for_each(|index| {
        //         dbg!("  index: {:?}", index);
        //     });
        // }

        // vector of all @unique/@@unique attributes in the given model
        // TODO: figure out how to retrieve these attributes
        let unique_sets: Vec<Vec<_>> = vec![];

        // vector of all @@index attributes in the given model
        // TODO: figure out how to retrieve these attributes
        let index_sets: Vec<Vec<_>> = vec![];

        let relation_fields_appear_in_unique = unique_sets
            .iter()
            .any(|unique_set| is_left_wise_included(&relation_fields, unique_set));

        let relation_fields_appear_in_index = index_sets
            .iter()
            .any(|index_set| is_left_wise_included(&relation_fields, index_set));

        if !(relation_fields_appear_in_unique || relation_fields_appear_in_index) {
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
