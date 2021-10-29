use itertools::Itertools;

use crate::{
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::walkers::ImplicitManyToManyRelationWalker,
};

/// Our weird many-to-many requirement.
pub(crate) fn validate_singular_id(relation: ImplicitManyToManyRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    for relation_field in [relation.field_a(), relation.field_b()].iter() {
        if !relation_field.related_model().has_single_id_field() {
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

            continue;
        }

        if !relation_field.references_singular_id_field() {
            diagnostics.push_error(DatamodelError::new_validation_error(
            &format!(
                "Many to many relations must always reference the id field of the related model. Change the argument `references` to use the id field of the related model `{}`. But it is referencing the following fields that are not the id: {}",
                &relation_field.related_model().name(),
                relation_field.referenced_fields().into_iter().flatten().map(|f| f.name()).join(", ")
            ),
            relation_field.ast_field().span)
        );
        }
    }
}
