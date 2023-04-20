use crate::validate::validation_pipeline::{context::Context, validations::relations::RELATION_ATTRIBUTE_NAME};
use crate::{datamodel_connector::ConnectorCapability, diagnostics::DatamodelError};
use parser_database::{ast::WithSpan, walkers::ImplicitManyToManyRelationWalker};

/// Our weird many-to-many requirement.
pub(crate) fn validate_singular_id(relation: ImplicitManyToManyRelationWalker<'_>, ctx: &mut Context<'_>) {
    for relation_field in [relation.field_a(), relation.field_b()].iter() {
        if !relation_field.related_model().has_single_id_field() {
            let container = if relation_field.related_model().ast_model().is_view() {
                "view"
            } else {
                "model"
            };

            let message = format!(
                "The relation field `{}` on {container} `{}` references `{}` which does not have an `@id` field. Models without `@id` cannot be part of a many to many relation. Use an explicit intermediate Model to represent this relationship.",
                &relation_field.name(),
                &relation_field.model().name(),
                &relation_field.related_model().name(),
            );

            ctx.push_error(DatamodelError::new_field_validation_error(
                &message,
                container,
                relation_field.model().name(),
                relation_field.name(),
                relation_field.ast_field().span(),
            ));

            continue;
        }

        if !relation_field.references_singular_id_field() {
            ctx.push_error(DatamodelError::new_validation_error(
            &format!(
                "Implicit many-to-many relations must always reference the id field of the related model. Change the argument `references` to use the id field of the related model `{}`. But it is referencing the following fields that are not the id: {}",
                &relation_field.related_model().name(),
                relation_field.referenced_fields().into_iter().flatten().map(|f| f.name()).collect::<Vec<_>>().join(", ")
            ),
            relation_field.ast_field().span())
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
        let msg = "Referential actions on implicit many-to-many relations are not supported";
        ctx.push_error(DatamodelError::new_validation_error(msg, span));
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
        .map(|r| r.ast_field().span());

    let msg = format!(
        "Implicit many-to-many relations are not supported on {}. Please use the syntax defined in https://pris.ly/d/document-database-many-to-many",
        ctx.connector.name()
    );

    for span in spans {
        ctx.push_error(DatamodelError::new_validation_error(&msg, span));
    }
}

pub(crate) fn cannot_define_references_argument(relation: ImplicitManyToManyRelationWalker<'_>, ctx: &mut Context<'_>) {
    let msg = "Implicit many-to-many relation should not have references argument defined. Either remove it, or change the relation to one-to-many.";

    if relation.field_a().referenced_fields().is_some() {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            msg,
            RELATION_ATTRIBUTE_NAME,
            relation.field_a().ast_field().span(),
        ));
    }

    if relation.field_b().referenced_fields().is_some() {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            msg,
            RELATION_ATTRIBUTE_NAME,
            relation.field_b().ast_field().span(),
        ));
    }
}
