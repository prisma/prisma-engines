pub(super) mod many_to_many;
pub(super) mod one_to_many;
pub(super) mod one_to_one;

mod visited_relation;

use crate::{
    ast,
    common::provider_names::MONGODB_SOURCE_NAME,
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::{
        walkers::{CompleteInlineRelationWalker, InlineRelationWalker, ReferencingFields},
        ConstraintName, ParserDatabase, ScalarFieldType,
    },
};
use datamodel_connector::{Connector, ConnectorCapability};
use itertools::Itertools;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
};
use visited_relation::*;

const PRISMA_FORMAT_HINT: &str = "You can run `prisma format` to fix this automatically.";
const RELATION_ATTRIBUTE_NAME: &str = "relation";
const RELATION_ATTRIBUTE_NAME_WITH_AT: &str = "@relation";
const STATE_ERROR: &str = "Failed lookup of model, field or optional property during internal processing. This means that the internal representation was mutated incorrectly.";

/// Depending on the database, a constraint name might need to be unique in a certain namespace.
/// Validates per database that we do not use a name that is already in use.
pub(crate) fn has_a_unique_constraint_name(
    db: &ParserDatabase<'_>,
    relation: CompleteInlineRelationWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    let name = match relation.foreign_key_name() {
        Some(name) => name,
        None => return,
    };

    let field = relation.referencing_field();
    let model = relation.referencing_model();

    for violation in db.scope_violations(model.model_id(), ConstraintName::Relation(name.as_ref())) {
        let span = field
            .ast_field()
            .span_for_argument("relation", "map")
            .unwrap_or_else(|| field.ast_field().span);

        let message = format!(
            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
            name,
            violation.description(model.name())
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            span,
        ));
    }
}

/// Required relational fields should point to required scalar fields.
pub(super) fn field_arity(relation: CompleteInlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    if !relation.referencing_field().ast_field().arity.is_required() {
        return;
    }

    if !relation.referencing_fields().any(|field| field.is_optional()) {
        return;
    }

    diagnostics.push_error(DatamodelError::new_validation_error(
        &format!(
            "The relation field `{}` uses the scalar fields {}. At least one of those fields is optional. Hence the relation field must be optional as well.",
            relation.referencing_field().name(),
            relation.referencing_fields().map(|field| field.name()).join(", "),
        ),
        relation.referencing_field().ast_field().span
    ));
}

/// The `fields` and `references` arguments should hold the same number of fields.
pub(super) fn same_length_in_referencing_and_referenced(
    relation: CompleteInlineRelationWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    if relation.referenced_fields().len() == 0 || relation.referencing_fields().len() == 0 {
        return;
    }

    if relation.referenced_fields().len() == relation.referencing_fields().len() {
        return;
    }

    let ast_field = relation.referencing_field().ast_field();
    let span = ast_field.span_for_attribute("relation").unwrap_or(ast_field.span);

    diagnostics.push_error(DatamodelError::new_validation_error(
        "You must specify the same number of fields in `fields` and `references`.",
        span,
    ));
}

/// Some connectors expect us to refer only unique fields from the foreign key.
pub(super) fn references_unique_fields(
    relation: CompleteInlineRelationWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    if relation.referenced_fields().len() == 0 || !diagnostics.errors().is_empty() {
        return;
    }

    if connector.supports_relations_over_non_unique_criteria() {
        return;
    }

    let references_unique_criteria = relation.referenced_model().unique_criterias().any(|criteria| {
        let mut criteria_field_names: Vec<_> = criteria.fields().map(|f| f.name()).collect();
        criteria_field_names.sort_unstable();

        let mut references_sorted: Vec<_> = relation.referenced_fields().map(|f| f.name()).collect();
        references_sorted.sort_unstable();

        criteria_field_names == references_sorted
    });

    if references_unique_criteria {
        return;
    }

    diagnostics.push_error(DatamodelError::new_validation_error(
        &format!(
            "The argument `references` must refer to a unique criteria in the related model `{}`. But it is referencing the following fields that are not a unique criteria: {}",
            relation.referenced_model().name(),
            relation.referenced_fields().map(|f| f.name()).join(", ")
        ),
        relation.referencing_field().ast_field().span
    ));
}

/// Some connectors want the fields and references in the same order.
pub(super) fn referencing_fields_in_correct_order(
    relation: CompleteInlineRelationWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    if relation.referenced_fields().len() == 0 || !diagnostics.errors().is_empty() {
        return;
    }

    if connector.allows_relation_fields_in_arbitrary_order() || relation.referenced_fields().len() == 1 {
        return;
    }

    let reference_order_correct = relation.referenced_model().unique_criterias().any(|criteria| {
        let criteria_fields = criteria.fields().map(|f| f.name());

        if criteria_fields.len() != relation.referenced_fields().len() {
            return false;
        }

        let references = relation.referenced_fields().map(|f| f.name());
        criteria_fields.zip(references).all(|(a, b)| a == b)
    });

    if reference_order_correct {
        return;
    }

    diagnostics.push_error(DatamodelError::new_validation_error(
        &format!(
            "The argument `references` must refer to a unique criteria in the related model `{}` using the same order of fields. Please check the ordering in the following fields: `{}`.",
            relation.referenced_model().name(),
            relation.referenced_fields().map(|f| f.name()).join(", ")
        ),
        relation.referencing_field().ast_field().span
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
pub(super) fn cycles<'ast, 'db>(
    relation: CompleteInlineRelationWalker<'ast, 'db>,
    db: &'db ParserDatabase<'ast>,
    diagnostics: &mut Diagnostics,
) {
    if !db
        .active_connector()
        .has_capability(ConnectorCapability::ReferenceCycleDetection)
        || db
            .datasource()
            .map(|ds| ds.referential_integrity().is_prisma())
            .unwrap_or(false)
    {
        return;
    }

    // poor man's tail-recursion ;)
    let mut next_relations = vec![(relation, Rc::new(VisitedRelation::root(relation)))];
    let parent_model = relation.referencing_model();

    while let Some((next_relation, visited_relations)) = next_relations.pop() {
        let related_model = next_relation.referenced_model();

        let on_delete = next_relation.on_delete();
        let on_update = next_relation.on_update();

        // a cycle has a meaning only if every relation in it triggers
        // modifications in the children
        if on_update.triggers_modification() || on_delete.triggers_modification() {
            let model = next_relation.referencing_model();

            if model == related_model {
                let msg = "A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes.";
                diagnostics.push_error(cascade_error_with_default_values(relation, msg));
                return;
            }

            if related_model == parent_model {
                let msg = format!(
                    "Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: {}.",
                    visited_relations
                );

                diagnostics.push_error(cascade_error_with_default_values(relation, &msg));
                return;
            }

            for relation in related_model.complete_inline_relations_from() {
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
pub(super) fn multiple_cascading_paths(
    relation: CompleteInlineRelationWalker<'_, '_>,
    db: &ParserDatabase<'_>,
    diagnostics: &mut Diagnostics,
) {
    let connector = db.active_connector();
    if !connector.has_capability(ConnectorCapability::ReferenceCycleDetection)
        || db
            .datasource()
            .map(|ds| ds.referential_integrity().is_prisma())
            .unwrap_or(false)
    {
        return;
    }

    if !relation.on_delete().triggers_modification() && !relation.on_update().triggers_modification() {
        return;
    }

    let mut visited = HashSet::new();
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
        .filter(|relation| relation.on_delete().triggers_modification() || relation.on_update().triggers_modification())
        .map(|relation| (relation, Rc::new(VisitedRelation::root(relation))))
        .collect();

    while let Some((next_relation, visited_relations)) = next_relations.pop() {
        let model = next_relation.referencing_model();
        let related_model = next_relation.referenced_model();

        visited.insert(next_relation.referencing_field());

        // Self-relations are detected elsewhere.
        if model == related_model {
            continue;
        }

        // Cycles are detected elsewhere.
        if related_model == parent_model {
            continue;
        }

        let mut forward_relations = related_model
            .complete_inline_relations_from()
            .filter(|relation| !visited.contains(&relation.referencing_field()))
            .map(|relation| (relation, Rc::new(visited_relations.link_next(relation))))
            .peekable();

        // If the related model does not have any paths to other models, we
        // traversed the whole path, storing it to the collection for later
        // inspection.
        if forward_relations.peek().is_none() {
            paths.push(visited_relations.link_next(next_relation));

            // We want to re-visit the same fields if coming from another path.
            visited.clear();

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

    let models = reachable
        .iter()
        .map(|model_name| format!("`{}`", model_name))
        .join(", ");

    #[allow(clippy::comparison_chain)] // match looks horrible here...
    if reachable.len() == 1 {
        let msg = format!(
            "When any of the records in model {} is updated or deleted, the referential actions on the relations cascade to model `{}` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`.",
            models,
            relation.referencing_model().name()
        );

        diagnostics.push_error(cascade_error_with_default_values(relation, &msg));
    } else if reachable.len() > 1 {
        let msg = format!(
            "When any of the records in models {} are updated or deleted, the referential actions on the relations cascade to model `{}` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`.",
            models,
            relation.referencing_model().name()
        );

        diagnostics.push_error(cascade_error_with_default_values(relation, &msg));
    }
}

fn cascade_error_with_default_values(relation: CompleteInlineRelationWalker<'_, '_>, msg: &str) -> DatamodelError {
    let on_delete = match relation.referencing_field().attributes().on_delete {
        None if relation.on_delete().triggers_modification() => Some(relation.on_delete()),
        _ => None,
    };

    let on_update = match relation.referencing_field().attributes().on_update {
        None if relation.on_update().triggers_modification() => Some(relation.on_update()),
        _ => None,
    };

    let mut msg = match (on_delete, on_update) {
        (Some(on_delete), Some(on_update)) => {
            format!(
                "{} (Implicit default `onDelete`: `{}`, and `onUpdate`: `{}`)",
                msg, on_delete, on_update
            )
        }
        (Some(on_delete), None) => {
            format!("{} (Implicit default `onDelete`: `{}`)", msg, on_delete)
        }
        (None, Some(on_update)) => {
            format!("{} (Implicit default `onUpdate`: `{}`)", msg, on_update)
        }
        (None, None) => msg.to_string(),
    };

    msg.push_str(" Read more at https://pris.ly/d/cyclic-referential-actions");

    DatamodelError::new_validation_error(&msg, relation.referencing_field().ast_field().span)
}

/// The types of the referencing and referenced scalar fields in a relation must be compatible.
pub(super) fn referencing_scalar_field_types(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let datasource = relation.db().datasource();
    // see https://github.com/prisma/prisma/issues/10105
    if datasource
        .as_ref()
        .map(|source| source.provider == MONGODB_SOURCE_NAME)
        .unwrap_or(false)
    {
        return;
    }

    let referencing_fields = match relation.referencing_fields() {
        ReferencingFields::Concrete(fields) => fields,
        _ => return,
    };

    for (referencing, referenced) in referencing_fields.zip(relation.referenced_fields()) {
        if !field_types_match(
            referencing.scalar_field.r#type,
            referenced.scalar_field.r#type,
            relation.db(),
        ) {
            diagnostics.push_error(DatamodelError::new_attribute_validation_error(
                &format!(
                    "The type of the field `{}` in the model `{}` is not matching the type of the referenced field `{}` in model `{}`.",
                    referencing.name(),
                    referencing.model().name(),
                    referenced.name(),
                    referenced.model().name(),
                ),
                RELATION_ATTRIBUTE_NAME,
                relation.forward_relation_field().unwrap().ast_field().span,
            ))
        }
    }

    fn field_types_match(referencing: ScalarFieldType, referenced: ScalarFieldType, db: &ParserDatabase<'_>) -> bool {
        match (referencing, referenced) {
            (ScalarFieldType::CompositeType(a), ScalarFieldType::CompositeType(b)) if a == b => true,
            (ScalarFieldType::Enum(a), ScalarFieldType::Enum(b)) if a == b => true,
            (ScalarFieldType::BuiltInScalar(a), ScalarFieldType::BuiltInScalar(b)) if a == b => true,
            (ScalarFieldType::Unsupported, ScalarFieldType::Unsupported) => true,
            (ScalarFieldType::Alias(a), b) => {
                let a_type = db.alias_scalar_field_type(&a);
                field_types_match(*a_type, b, db)
            }
            (a, ScalarFieldType::Alias(b)) => {
                let b_type = db.alias_scalar_field_type(&b);
                field_types_match(a, *b_type, db)
            }
            _ => false,
        }
    }
}

fn is_empty_fields(fields: Option<&[ast::FieldId]>) -> bool {
    match fields {
        None | Some([]) => true,
        Some(_) => false,
    }
}
