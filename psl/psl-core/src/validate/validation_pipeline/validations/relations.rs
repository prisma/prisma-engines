pub(super) mod many_to_many;
pub(super) mod one_to_many;
pub(super) mod one_to_one;

mod visited_relation;

use super::constraint_namespace::ConstraintName;
use crate::datamodel_connector::{walker_ext_traits::*, Connector, ConnectorCapability, RelationMode};
use crate::{diagnostics::DatamodelError, validate::validation_pipeline::context::Context};
use diagnostics::DatamodelWarning;
use indoc::formatdoc;
use itertools::Itertools;
use parser_database::walkers::RelationFieldId;
use parser_database::ReferentialAction;
use parser_database::{
    ast::WithSpan,
    walkers::{CompleteInlineRelationWalker, InlineRelationWalker},
    ScalarFieldType,
};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
};
use visited_relation::*;

const PRISMA_FORMAT_HINT: &str = "You can run `prisma format` to fix this automatically.";
const RELATION_ATTRIBUTE_NAME: &str = "@relation";
const STATE_ERROR: &str = "Failed lookup of model, field or optional property during internal processing. This means that the internal representation was mutated incorrectly.";

/// Depending on the database, a constraint name might need to be unique in a certain namespace.
/// Validates per database that we do not use a name that is already in use.
pub(super) fn has_a_unique_constraint_name(
    names: &super::Names<'_>,
    relation: InlineRelationWalker<'_>,
    ctx: &mut Context<'_>,
) {
    let name = relation.constraint_name(ctx.connector);
    let model = relation.referencing_model();

    for violation in names.constraint_namespace.constraint_name_scope_violations(
        model.model_id(),
        ConstraintName::Relation(name.as_ref()),
        ctx,
    ) {
        let span = relation
            .forward_relation_field()
            .map(|rf| {
                rf.ast_field()
                    .span_for_argument("relation", "map")
                    .unwrap_or_else(|| rf.ast_field().span())
            })
            .unwrap_or_else(|| relation.referenced_model().ast_model().span());

        let message = format!(
            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
            name,
            violation.description(model.name())
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            span,
        ));
    }
}

/// Required relational fields should point to required scalar fields.
pub(super) fn field_arity(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let forward_relation_field = if let Some(f) = relation.forward_relation_field() {
        f
    } else {
        return;
    };

    if !forward_relation_field.ast_field().arity.is_required() {
        return;
    }

    match relation.referencing_fields() {
        Some(mut fields) => {
            if fields.all(|field| !field.is_optional()) {
                return;
            }
        }
        _ => return,
    }
    let scalar_field_names: Vec<&str> = relation.referencing_fields().unwrap().map(|f| f.name()).collect();

    ctx.push_error(DatamodelError::new_validation_error(
        &format!(
            "The relation field `{}` uses the scalar fields {}. At least one of those fields is optional. Hence the relation field must be optional as well.",
            forward_relation_field.name(),
            scalar_field_names.join(", "),
        ),
        forward_relation_field.ast_field().span()
    ));
}

/// The `fields` and `references` arguments should hold the same number of fields.
pub(super) fn same_length_in_referencing_and_referenced(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let relation_field = if let Some(forward) = relation.forward_relation_field() {
        forward
    } else {
        return;
    };

    match (relation_field.referencing_fields(), relation_field.referenced_fields()) {
        (Some(fields), Some(references)) if fields.len() != references.len() => {
            ctx.push_error(DatamodelError::new_validation_error(
                "You must specify the same number of fields in `fields` and `references`.",
                relation_field.relation_attribute().unwrap().span,
            ));
        }
        _ => (),
    }
}

/// Some connectors expect us to refer only unique fields from the foreign key.
pub(super) fn references_unique_fields(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let relation_field = if let Some(rf) = relation.forward_relation_field() {
        rf
    } else {
        return;
    };

    if relation_field.referenced_fields().map(|f| f.len() == 0).unwrap_or(true) {
        return;
    }

    let references_unique_criterion = relation.referenced_model().unique_criterias().any(|criteria| {
        let mut criteria_field_names: Vec<_> = criteria.fields().map(|f| f.name()).collect();
        criteria_field_names.sort_unstable();

        let mut references_sorted: Vec<_> = relation.referenced_fields().map(|f| f.name()).collect();
        references_sorted.sort_unstable();

        criteria_field_names == references_sorted
    });

    if references_unique_criterion {
        referencing_fields_in_correct_order(relation, ctx);
        return;
    }

    let fields: Vec<_> = relation.referenced_fields().map(|f| f.name()).collect();
    let model = relation.referenced_model().name();

    let message = if fields.len() == 1 {
        format!("The argument `references` must refer to a unique criterion in the related model. Consider adding an `@unique` attribute to the field `{}` in the model `{}`.", fields.join(", "), model)
    } else {
        format!("The argument `references` must refer to a unique criterion in the related model. Consider adding an `@@unique([{}])` attribute to the model `{}`.", fields.join(", "), model)
    };

    ctx.push_error(DatamodelError::new_attribute_validation_error(
        &message,
        RELATION_ATTRIBUTE_NAME,
        relation_field.ast_field().span(),
    ));
}

/// Most connectors want the fields and references in the same order.
fn referencing_fields_in_correct_order(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let relation_field = if let Some(rf) = relation.forward_relation_field() {
        rf
    } else {
        return;
    };

    if relation_field
        .referenced_fields()
        .map(|fields| fields.len() <= 1)
        .unwrap_or(true)
    {
        return;
    }

    if ctx.connector.allows_relation_fields_in_arbitrary_order() {
        return;
    }

    let reference_order_correct = relation.referenced_model().unique_criterias().any(|criteria| {
        let criteria_fields = criteria.fields().map(|f| f.name());

        if criteria_fields.len()
            != relation_field
                .referenced_fields()
                .map(|fields| fields.len())
                .unwrap_or(0)
        {
            return false;
        }

        let references = relation.referenced_fields().map(|f| f.name());
        criteria_fields.zip(references).all(|(a, b)| a == b)
    });

    if reference_order_correct {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        &format!(
            "The argument `references` must refer to a unique criterion in the related model `{}` using the same order of fields. Please check the ordering in the following fields: `{}`.",
            relation.referenced_model().name(),
            relation.referenced_fields().map(|f| f.name()).join(", ")
        ),
        relation_field.ast_field().span()
    ));
}

/// Detects cyclical cascading referential actions. Counts as a cycle if and
/// only if all relations have at least one action triggering a cascading
/// behavior, which is anything other than `NoAction`.
///
/// # Examples
///
/// A -> A (self relation)
/// A -> B -> A (cycle)
///
/// We count them from forward-relations, e.g. from the side that defines the
/// foreign key. Many to many relations we skip. The user must set one of the
/// relation links to NoAction for both referential actions.
pub(super) fn cycles(relation: CompleteInlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx
        .connector
        .has_capability(ConnectorCapability::ReferenceCycleDetection)
        && ctx
            .datasource
            .map(|ds| ds.relation_mode().uses_foreign_keys())
            .unwrap_or(true)
    {
        return;
    }

    let mut visited = HashSet::new();

    // poor man's tail-recursion ;)
    let mut next_relations = vec![(relation, Rc::new(VisitedRelation::root(relation)))];
    let parent_model = relation.referencing_model();

    while let Some((next_relation, visited_relations)) = next_relations.pop() {
        visited.insert(next_relation.referencing_field().id);

        let related_model = next_relation.referenced_model();

        let on_delete = next_relation.on_delete(ctx.connector, ctx.relation_mode);
        let on_update = next_relation.on_update();

        // a cycle has a meaning only if every relation in it triggers
        // modifications in the children
        if on_update.triggers_modification() || on_delete.triggers_modification() {
            let model = next_relation.referencing_model();

            if model.id == related_model.id {
                let msg = "A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes.";
                ctx.push_error(cascade_error_with_default_values(
                    relation,
                    ctx.connector,
                    ctx.relation_mode,
                    msg,
                ));

                return;
            }

            if related_model.id == parent_model.id {
                let msg = format!(
                    "Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: {visited_relations}."
                );

                ctx.push_error(cascade_error_with_default_values(
                    relation,
                    ctx.connector,
                    ctx.relation_mode,
                    &msg,
                ));
                return;
            }

            let relations = related_model
                .complete_inline_relations_from()
                .filter(|r| !visited.contains(&r.referencing_field().id));

            for relation in relations {
                next_relations.push((relation, Rc::new(visited_relations.link_next(relation))));
            }
        }
    }
}

/// From the given relation, checks if any other relation fits the criteria:
///
/// - Triggers a cascading action (anything else but NoAction)
/// - Refers to the same model as the given relation at some point in the path
///
/// # Example
///
/// - A -> B -> C
/// - A -> D -> C
///
/// The user must set one of these relations to use NoAction for onUpdate and
/// onDelete.
pub(super) fn multiple_cascading_paths(relation: CompleteInlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx
        .connector
        .has_capability(ConnectorCapability::ReferenceCycleDetection)
        || ctx.relation_mode.is_prisma()
    {
        return;
    }

    let triggers_modifications = |relation: &CompleteInlineRelationWalker<'_>| {
        relation
            .on_delete(ctx.connector, ctx.relation_mode)
            .triggers_modification()
            || relation.on_update().triggers_modification()
    };

    if !triggers_modifications(&relation) {
        return;
    }

    let parent_model = relation.referencing_model();

    // Gather all paths from this model to any other model, skipping
    // cyclical relations
    let mut paths = Vec::new();

    // Add all relations from current model to the graph vector. At this point
    // we only care about multiple paths from this model to any other model. We
    // handle paths that cross, but are not started from here, when calling the
    // function from corresponding models.
    let mut next_relations: Vec<_> = relation
        .referencing_model()
        .complete_inline_relations_from()
        .filter(triggers_modifications)
        .map(|relation| {
            (
                relation,
                Rc::new(VisitedRelation::root(relation)),
                HashSet::<RelationFieldId>::new(),
            )
        })
        .collect();

    while let Some((next_relation, visited_relations, mut current_path)) = next_relations.pop() {
        let model = next_relation.referencing_model();
        let related_model = next_relation.referenced_model();

        current_path.insert(next_relation.referencing_field().id);

        // Self-relations are detected elsewhere.
        if model.id == related_model.id {
            continue;
        }

        // Cycles are detected elsewhere.
        if related_model.id == parent_model.id {
            continue;
        }

        let mut forward_relations = related_model
            .complete_inline_relations_from()
            .filter(triggers_modifications)
            .filter(|relation| !current_path.contains(&relation.referencing_field().id))
            .map(|relation| {
                (
                    relation,
                    Rc::new(visited_relations.link_next(relation)),
                    current_path.clone(),
                )
            })
            .peekable();

        // If the related model does not have any paths to other models, we
        // traversed the whole path, storing it to the collection for later
        // inspection.
        if forward_relations.peek().is_none() {
            paths.push(visited_relations.link_next(next_relation));

            continue;
        }

        next_relations.extend(forward_relations);
    }

    // Gather all paths from a relation field to all the models seen from that
    // field.
    let mut seen: HashMap<&str, HashSet<&str>> = HashMap::new();
    for path in paths.iter() {
        let mut iter = path.iter().peekable();

        let seen_models = match iter.peek() {
            Some(relation) => seen.entry(relation.referencing_field().name()).or_default(),
            _ => continue,
        };

        for relation in iter {
            seen_models.insert(relation.referenced_model().name());
        }
    }

    // Compact the model nameas that can be reached from the given relation
    // field, but also from any other relation field in the same model.
    let mut reachable: BTreeSet<&str> = BTreeSet::new();

    if let Some(from_parent) = seen.remove(relation.referencing_field().name()) {
        for (_, from_other) in seen.into_iter() {
            reachable.extend(from_parent.intersection(&from_other));
        }
    }

    let models = reachable.iter().map(|model_name| format!("`{model_name}`")).join(", ");

    #[allow(clippy::comparison_chain)] // match looks horrible here...
    if reachable.len() == 1 {
        let msg = format!(
            "When any of the records in model {} is updated or deleted, the referential actions on the relations cascade to model `{}` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`.",
            models,
            relation.referencing_model().name()
        );

        ctx.push_error(cascade_error_with_default_values(
            relation,
            ctx.connector,
            ctx.relation_mode,
            &msg,
        ));
    } else if reachable.len() > 1 {
        let msg = format!(
            "When any of the records in models {} are updated or deleted, the referential actions on the relations cascade to model `{}` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`.",
            models,
            relation.referencing_model().name()
        );

        ctx.push_error(cascade_error_with_default_values(
            relation,
            ctx.connector,
            ctx.relation_mode,
            &msg,
        ));
    }
}

fn cascade_error_with_default_values(
    relation: CompleteInlineRelationWalker<'_>,
    connector: &dyn Connector,
    relation_mode: RelationMode,
    msg: &str,
) -> DatamodelError {
    let on_delete = match relation.referencing_field().explicit_on_delete() {
        None if relation.on_delete(connector, relation_mode).triggers_modification() => {
            Some(relation.on_delete(connector, relation_mode))
        }
        _ => None,
    };

    let on_update = match relation.referencing_field().explicit_on_update() {
        None if relation.on_update().triggers_modification() => Some(relation.on_update()),
        _ => None,
    };

    let mut msg = match (on_delete, on_update) {
        (Some(on_delete), Some(on_update)) => {
            format!(
                "{} (Implicit default `onDelete`: `{}`, and `onUpdate`: `{}`)",
                msg,
                on_delete.as_str(),
                on_update.as_str()
            )
        }
        (Some(on_delete), None) => {
            format!("{} (Implicit default `onDelete`: `{}`)", msg, on_delete.as_str())
        }
        (None, Some(on_update)) => {
            format!("{} (Implicit default `onUpdate`: `{}`)", msg, on_update.as_str())
        }
        (None, None) => msg.to_string(),
    };

    msg.push_str(" Read more at https://pris.ly/d/cyclic-referential-actions");

    DatamodelError::new_validation_error(&msg, relation.referencing_field().ast_field().span())
}

/// The types of the referencing and referenced scalar fields in a relation must be compatible.
pub(super) fn referencing_scalar_field_types(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let referencing_fields = if let Some(fields) = relation.referencing_fields() {
        fields
    } else {
        return;
    };

    for (referencing, referenced) in referencing_fields.zip(relation.referenced_fields()) {
        if !field_types_match(referencing.scalar_field_type(), referenced.scalar_field_type()) {
            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &format!(
                    "The type of the field `{}` in the model `{}` is not matching the type of the referenced field `{}` in model `{}`.",
                    referencing.name(),
                    referencing.model().name(),
                    referenced.name(),
                    referenced.model().name(),
                ),
                RELATION_ATTRIBUTE_NAME,
                relation.forward_relation_field().unwrap().ast_field().span(),
            ))
        }
    }

    fn field_types_match(referencing: ScalarFieldType, referenced: ScalarFieldType) -> bool {
        match (referencing, referenced) {
            (ScalarFieldType::CompositeType(a), ScalarFieldType::CompositeType(b)) if a == b => true,
            (ScalarFieldType::Enum(a), ScalarFieldType::Enum(b)) if a == b => true,
            (ScalarFieldType::BuiltInScalar(a), ScalarFieldType::BuiltInScalar(b)) if a == b => true,
            (ScalarFieldType::Unsupported(a), ScalarFieldType::Unsupported(b)) if a == b => true,
            _ => false,
        }
    }
}

fn is_empty_fields<T>(fields: Option<impl ExactSizeIterator<Item = T>>) -> bool {
    match fields {
        None => true,
        Some(fields) => fields.len() == 0,
    }
}

/// There cannot be any required field in a relation where one of the referential actions is SetNull.
pub(crate) fn required_relation_cannot_use_set_null(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    // return early if there's no relation field on the referencing model
    let forward = match relation.forward_relation_field() {
        Some(forward) => forward,
        None => return,
    };

    // return early if no referencing field is required
    if forward
        .referencing_fields()
        .map(|mut fields| fields.all(|f| !f.ast_field().arity.is_required()))
        .unwrap_or_default()
    {
        return;
    }

    let span = forward.ast_field().span();

    if ctx
        .connector
        .allows_set_null_referential_action_on_non_nullable_fields(ctx.relation_mode)
    {
        // the database allows SetNull on non-nullable fields, we add a validation warning to avoid breaking changes
        let warning_template = |referential_action_type: &str| {
            formatdoc!(
                    r#"
                    The `{referential_action_type}` referential action of a relation should not be set to `{set_null}` when a referenced field is required.
                    We recommend either to choose another referential action, or to make the referenced fields optional.
                    Read more at https://pris.ly/d/postgres-set-null
                    "#,
                    set_null = ReferentialAction::SetNull.as_str(),
                ).replace('\n', " ")
        };

        if let Some(ReferentialAction::SetNull) = forward.explicit_on_delete() {
            ctx.push_warning(DatamodelWarning::new(warning_template("onDelete"), span))
        }

        if let Some(ReferentialAction::SetNull) = forward.explicit_on_update() {
            ctx.push_warning(DatamodelWarning::new(warning_template("onUpdate"), span))
        }
    } else {
        // the database allows does not allow SetNull on non-nullable fields, we add a validation error
        let error_template = |referential_action_type: &str| {
            let set_null = ReferentialAction::SetNull.as_str();
            let msg = formatdoc! {r#"
                The `{referential_action_type}` referential action of a relation must not be set to `{set_null}` when a referenced field is required.
                Either choose another referential action, or make the referenced fields optional.
            "#};
            msg
        };

        if let Some(ReferentialAction::SetNull) = forward.explicit_on_delete() {
            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &error_template("onDelete"),
                RELATION_ATTRIBUTE_NAME,
                span,
            ))
        }

        if let Some(ReferentialAction::SetNull) = forward.explicit_on_update() {
            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &error_template("onUpdate"),
                RELATION_ATTRIBUTE_NAME,
                span,
            ))
        }
    }
}
