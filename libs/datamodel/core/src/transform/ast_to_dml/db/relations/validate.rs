mod visited_relation;

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
};

use datamodel_connector::{Connector, ConnectorCapability};
use itertools::Itertools;
use visited_relation::*;

use crate::{diagnostics::DatamodelError, transform::ast_to_dml::db::walkers::ExplicitRelationWalker};

/// The `fields` and `references` should hold the same number of fields.
pub(super) fn same_length_in_referencing_and_referenced(
    relation: ExplicitRelationWalker<'_, '_>,
    errors: &mut Vec<DatamodelError>,
) {
    if relation.referenced_fields().len() == 0 || relation.referencing_fields().len() == 0 {
        return;
    }

    if relation.referenced_fields().len() == relation.referencing_fields().len() {
        return;
    }

    let ast_field = relation.referencing_field().ast_field();
    let span = ast_field.span_for_attribute("relation").unwrap_or(ast_field.span);

    errors.push(DatamodelError::new_validation_error(
        "You must specify the same number of fields in `fields` and `references`.",
        span,
    ));
}

/// Some connectors expect us to refer only unique fields from the foreign key.
pub(super) fn references_unique_fields(
    relation: ExplicitRelationWalker<'_, '_>,
    connector: &dyn Connector,
    errors: &mut Vec<DatamodelError>,
) {
    if relation.referenced_fields().len() == 0 || !errors.is_empty() {
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

    errors.push(DatamodelError::new_validation_error(
        &format!(
            "The argument `references` must refer to a unique criteria in the related model `{}`. But it is referencing the following fields that are not a unique criteria: {}",
            relation.referenced_model().name(),
            relation.referenced_fields().map(|f| f.name()).join(", ")
        ),
        relation.referencing_field().ast_field().span
    ));
}

/// Some connectors want the fields and references in the same order, and some
/// other connectors wants foreign keys to point to unique criterias.
pub(super) fn referencing_fields_in_correct_order(
    relation: ExplicitRelationWalker<'_, '_>,
    connector: &dyn Connector,
    errors: &mut Vec<DatamodelError>,
) {
    if relation.referenced_fields().len() == 0 || !errors.is_empty() {
        return;
    }

    if connector.allows_relation_fields_in_arbitrary_order() || !relation.is_compound() {
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

    errors.push(DatamodelError::new_validation_error(
        &format!(
            "The argument `references` must refer to a unique criteria in the related model `{}` using the same order of fields. Please check the ordering in the following fields: `{}`.",
            relation.referenced_model().name(),
            relation.referenced_fields().map(|f| f.name()).join(", ")
        ),
        relation.referencing_field().ast_field().span
    ));
}

/// Validate that the arity of fields from `fields` is compatible with relation field arity.
pub(super) fn field_arity(relation: ExplicitRelationWalker<'_, '_>, errors: &mut Vec<DatamodelError>) {
    if !relation.referencing_field().ast_field().arity.is_required() {
        return;
    }

    if !relation.referencing_fields().any(|field| field.is_optional()) {
        return;
    }

    errors.push(DatamodelError::new_validation_error(
        &format!(
            "The relation field `{}` uses the scalar fields {}. At least one of those fields is optional. Hence the relation field must be optional as well.",
            relation.referencing_field().name(),
            relation.referencing_fields().map(|field| field.name()).join(", "),
        ),
        relation.referencing_field().ast_field().span
    ));
}

/// Detects cyclical cascading referential actions.
pub(super) fn detect_cycles<'ast, 'db>(
    relation: ExplicitRelationWalker<'ast, 'db>,
    connector: &dyn Connector,
    errors: &mut Vec<DatamodelError>,
) {
    if !connector.has_capability(ConnectorCapability::ReferenceCycleDetection)
        || !connector.has_capability(ConnectorCapability::ForeignKeys)
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

            if model.model_id == related_model.model_id {
                let msg = "A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes.";
                errors.push(cascade_error_with_default_values(relation, msg));
                return;
            }

            if related_model.model_id == parent_model.model_id {
                let msg = format!(
                    "Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: {}.",
                    visited_relations
                );

                errors.push(cascade_error_with_default_values(relation, &msg));
                return;
            }

            for relation in related_model.explicit_forward_relations() {
                next_relations.push((relation, Rc::new(visited_relations.link_next(relation))));
            }
        }
    }
}

pub(super) fn detect_multiple_cascading_paths(
    relation: ExplicitRelationWalker<'_, '_>,
    connector: &dyn Connector,
    errors: &mut Vec<DatamodelError>,
) {
    if !connector.has_capability(ConnectorCapability::ReferenceCycleDetection)
        || !connector.has_capability(ConnectorCapability::ForeignKeys)
    {
        return;
    }

    if !relation.on_delete().triggers_modification() && !relation.on_update().triggers_modification() {
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
        .explicit_forward_relations()
        .filter(|relation| relation.on_delete().triggers_modification() || relation.on_update().triggers_modification())
        .map(|relation| (relation, Rc::new(VisitedRelation::root(relation))))
        .collect();

    while let Some((next_relation, visited_relations)) = next_relations.pop() {
        let model = next_relation.referencing_model();
        let related_model = next_relation.referenced_model();

        // Self-relations are detected elsewhere.
        if model.model_id == related_model.model_id {
            continue;
        }

        // Cycles are detected elsewhere.
        if related_model.model_id == parent_model.model_id {
            continue;
        }

        let mut forward_relations = related_model
            .explicit_forward_relations()
            .map(|relation| (relation, Rc::new(visited_relations.link_next(relation))))
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

    let models = reachable
        .iter()
        .map(|model_name| format!("`{}`", model_name))
        .join(", ");

    if reachable.len() == 1 {
        let msg = format!(
            "When any of the records in model {} is updated or deleted, the referential actions on the relations cascade to model `{}` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`.",
            models,
            relation.referencing_model().name()
        );

        errors.push(cascade_error_with_default_values(relation, &msg));
    } else if reachable.len() > 1 {
        let msg = format!(
            "When any of the records in models {} are updated or deleted, the referential actions on the relations cascade to model `{}` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`.",
            models,
            relation.referencing_model().name()
        );

        errors.push(cascade_error_with_default_values(relation, &msg));
    }
}

fn cascade_error_with_default_values(relation: ExplicitRelationWalker<'_, '_>, msg: &str) -> DatamodelError {
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
