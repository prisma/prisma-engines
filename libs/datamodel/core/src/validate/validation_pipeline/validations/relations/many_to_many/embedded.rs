use crate::datamodel_connector::ConnectorCapability;
use crate::{diagnostics::DatamodelError, validate::validation_pipeline::context::Context};
use parser_database::{ast::WithSpan, walkers::TwoWayEmbeddedManyToManyRelationWalker};

/// Only MongoDb should support embedded M:N relations.
pub(crate) fn supports_embedded_relations(relation: TwoWayEmbeddedManyToManyRelationWalker<'_>, ctx: &mut Context<'_>) {
    if ctx
        .connector
        .has_capability(ConnectorCapability::TwoWayEmbeddedManyToManyRelation)
    {
        return;
    }

    let spans = [relation.field_a(), relation.field_b()]
        .into_iter()
        .map(|r| r.ast_field().span());

    let connector_name = ctx.connector.name();

    let msg = format!(
        "Embedded many-to-many relations are not supported on {connector_name}. Please use the syntax defined in https://pris.ly/d/relational-database-many-to-many",
    );

    for span in spans {
        ctx.push_error(DatamodelError::new_validation_error(&msg, span));
    }
}

/// Both relation fields must have `references` argument that points to an exactly one scalar field.
pub(crate) fn defines_references_on_both_sides(
    relation: TwoWayEmbeddedManyToManyRelationWalker<'_>,
    ctx: &mut Context<'_>,
) {
    if !ctx
        .connector
        .has_capability(ConnectorCapability::TwoWayEmbeddedManyToManyRelation)
    {
        return;
    }

    let spans = [relation.field_a(), relation.field_b()]
        .into_iter()
        .filter_map(|r| match r.referenced_fields() {
            Some(fields) if fields.len() == 1 => None,
            _ => {
                let ast_field = r.ast_field();

                let span = ast_field
                    .span_for_argument("relation", "references")
                    .or_else(|| ast_field.span_for_attribute("relation"))
                    .unwrap_or_else(|| ast_field.span());

                Some(span)
            }
        });

    let msg = "The `references` argument must be defined and must point to exactly one scalar field. https://pris.ly/d/many-to-many-relations";

    for span in spans {
        ctx.push_error(DatamodelError::new_attribute_validation_error(msg, "@relation", span));
    }
}

/// Both sides must define the `fields` argument and the argument must point to
/// exactly one scalar field.
pub(crate) fn defines_fields_on_both_sides(
    relation: TwoWayEmbeddedManyToManyRelationWalker<'_>,
    ctx: &mut Context<'_>,
) {
    if !ctx
        .connector
        .has_capability(ConnectorCapability::TwoWayEmbeddedManyToManyRelation)
    {
        return;
    }

    let spans = [relation.field_a(), relation.field_b()]
        .into_iter()
        .filter_map(|r| match r.referencing_fields() {
            Some(fields) if fields.len() == 1 => None,
            _ => {
                let ast_field = r.ast_field();

                let span = ast_field
                    .span_for_argument("relation", "fields")
                    .or_else(|| ast_field.span_for_attribute("relation"))
                    .unwrap_or_else(|| ast_field.span());

                Some(span)
            }
        });

    let msg = "The `fields` argument must be defined and must point to exactly one scalar field. https://pris.ly/d/many-to-many-relations";

    for span in spans {
        ctx.push_error(DatamodelError::new_attribute_validation_error(msg, "@relation", span));
    }
}

/// We only support referencing an id field, no uniques or no normal fields allowed.
pub(crate) fn references_id_from_both_sides(
    relation: TwoWayEmbeddedManyToManyRelationWalker<'_>,
    ctx: &mut Context<'_>,
) {
    if !ctx
        .connector
        .has_capability(ConnectorCapability::TwoWayEmbeddedManyToManyRelation)
    {
        return;
    }

    let spans = [relation.field_a(), relation.field_b()].into_iter().filter_map(|r| {
        match r.referenced_fields().and_then(|mut r| r.next()) {
            Some(field) if !field.is_single_pk() => {
                let span = r
                    .ast_field()
                    .span_for_argument("relation", "references")
                    .unwrap_or_else(|| r.ast_field().span());

                Some(span)
            }
            _ => None,
        }
    });

    let msg = "The `references` argument must point to a singular `id` field";

    for span in spans {
        ctx.push_error(DatamodelError::new_attribute_validation_error(msg, "@relation", span));
    }
}

/// The `fields` argument has one scalar field of an array type. The
/// `references` argument must be of same type, but not an array.
pub(crate) fn referencing_with_an_array_field_of_correct_type(
    relation: TwoWayEmbeddedManyToManyRelationWalker<'_>,
    ctx: &mut Context<'_>,
) {
    if !ctx
        .connector
        .has_capability(ConnectorCapability::TwoWayEmbeddedManyToManyRelation)
    {
        return;
    }

    let error_msg =
        "The scalar field defined in `fields` argument must be an array of the same type defined in `references`";

    for field in [relation.field_a(), relation.field_b()] {
        let referencing = field.referencing_fields().and_then(|mut r| r.next());

        let referenced = field.referenced_fields().and_then(|mut r| r.next());

        let (referencing, referenced) = match (referencing, referenced) {
            (Some(fields), Some(references)) => (fields, references),
            _ => continue,
        };

        let types_match = referencing.scalar_field_type() == referenced.scalar_field_type();
        let references_a_singular_field = !referenced.ast_field().arity.is_list();

        let raw_types_match = referencing.raw_native_type().map(|r| r.1) == referenced.raw_native_type().map(|r| r.1);

        if referencing.ast_field().arity.is_list() && types_match && raw_types_match && references_a_singular_field {
            continue;
        }

        let ast_field = field.ast_field();
        let span = ast_field
            .span_for_attribute("relation")
            .unwrap_or_else(|| ast_field.span());

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            error_msg,
            "@relation",
            span,
        ));
    }
}

/// No referential actions allowed in embedded 2-way relations (yet).
pub(crate) fn validate_no_referential_actions(
    relation: TwoWayEmbeddedManyToManyRelationWalker<'_>,
    ctx: &mut Context<'_>,
) {
    if !ctx
        .connector
        .has_capability(ConnectorCapability::TwoWayEmbeddedManyToManyRelation)
    {
        return;
    }

    let referential_action_spans = [relation.field_a(), relation.field_b()].into_iter().flat_map(|field| {
        field
            .explicit_on_delete_span()
            .into_iter()
            .chain(field.explicit_on_update_span().into_iter())
    });

    for span in referential_action_spans {
        let msg = "Referential actions on two-way embedded many-to-many relations are not supported";
        ctx.push_error(DatamodelError::new_validation_error(msg, span));
    }
}
