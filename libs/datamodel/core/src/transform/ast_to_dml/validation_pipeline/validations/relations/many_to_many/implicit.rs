use datamodel_connector::{ConnectorCapability, DatamodelError};
use itertools::Itertools;
use parser_database::walkers::ImplicitManyToManyRelationWalker;

use crate::transform::ast_to_dml::validation_pipeline::context::Context;

/// Our weird many-to-many requirement.
pub(crate) fn validate_singular_id(relation: ImplicitManyToManyRelationWalker<'_>, ctx: &mut Context<'_>) {
    for relation_field in [relation.field_a(), relation.field_b()].iter() {
        if !relation_field.related_model().has_single_id_field() {
            let message = format!(
                "The relation field `{}` on Model `{}` references `{}` which does not have an `@id` field. Models without `@id` cannot be part of a many to many relation. Use an explicit intermediate Model to represent this relationship.",
                &relation_field.name(),
                &relation_field.model().name(),
                &relation_field.related_model().name(),
            );

            ctx.push_error(DatamodelError::new_field_validation_error(
                &message,
                relation_field.model().name(),
                relation_field.name(),
                relation_field.ast_field().span,
            ));

            continue;
        }

        if !relation_field.references_singular_id_field() {
            ctx.push_error(DatamodelError::new_validation_error(
            format!(
                "Implicit many-to-many relations must always reference the id field of the related model. Change the argument `references` to use the id field of the related model `{}`. But it is referencing the following fields that are not the id: {}",
                &relation_field.related_model().name(),
                relation_field.referenced_fields().into_iter().flatten().map(|f| f.name()).join(", ")
            ),
            relation_field.ast_field().span)
        );
        }
    }
}

/// M:N relations cannot have referential actions defined (yet).
pub(crate) fn validate_no_referential_actions(relation: ImplicitManyToManyRelationWalker<'_>, ctx: &mut Context<'_>) {
    let referential_action_spans = [relation.field_a(), relation.field_b()].into_iter().flat_map(|field| {
        field
            .explicit_on_delete_span()
            .into_iter()
            .chain(field.explicit_on_update_span().into_iter())
    });

    for span in referential_action_spans {
        ctx.push_error(DatamodelError::new_validation_error(
            "Referential actions on implicit many-to-many relations are not supported".to_owned(),
            span,
        ));
    }
}

/// We do not support implicit m:n relations on MongoDb.
pub(crate) fn supports_implicit_relations(relation: ImplicitManyToManyRelationWalker<'_>, ctx: &mut Context<'_>) {
    if ctx
        .connector
        .has_capability(ConnectorCapability::ImplicitManyToManyRelation)
    {
        return;
    }

    let spans = [relation.field_a(), relation.field_b()]
        .into_iter()
        .map(|r| r.ast_field().span);

    let msg = format!(
        "Implicit many-to-many relations are not supported on {}. Please use the syntax defined in https://pris.ly/d/document-database-many-to-many",
        ctx.connector.name()
    );

    for span in spans {
        ctx.push_error(DatamodelError::new_validation_error(msg.clone(), span));
    }
}
