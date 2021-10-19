use crate::{
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::walkers::ImplicitManyToManyRelationWalker,
};

pub(crate) fn validate_strict(relation: ImplicitManyToManyRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    for relation_field in [relation.field_a(), relation.field_b()]
        .iter()
        .filter(|field| !field.related_model().has_single_id_field())
    {
        let message = format!(
            "The relation field `{}` on Model `{}` references `{}` which does not have an `@id` field. Models without `@id` cannot be part of a many to many relation. Use an explicit intermediate Model to represent this relationship.",
            &relation_field.name(),
            &relation_field.model().name(),
            &relation_field.related_model().name(),
        );

        diagnostics.push_error(DatamodelError::new_field_validation_error(
            &message,
            relation_field.model().name(),
            relation_field.name(),
            relation_field.ast_field().span,
        ));
    }
}
