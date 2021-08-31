mod visited_relation;

use crate::{
    ast::Span,
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::validate::RELATION_ATTRIBUTE_NAME,
};
use dml::{datamodel::Datamodel, field::RelationField, model::Model, traits::WithName};
use itertools::Itertools;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
};
use visited_relation::VisitedRelation;

/// Find relations from the given model that end up into the same model,
/// triggering cascading referential actions.
pub(crate) fn detect_multiple_cascading_paths(
    datamodel: &Datamodel,
    parent_model: &Model,
    parent_field: &RelationField,
    span: Span,
    errors: &mut Diagnostics,
) {
    let on_update = parent_field
        .relation_info
        .on_update
        .unwrap_or_else(|| parent_field.default_on_update_action());

    let on_delete = parent_field
        .relation_info
        .on_delete
        .unwrap_or_else(|| parent_field.default_on_delete_action());

    if !on_update.triggers_modification() && !on_delete.triggers_modification() {
        return;
    }

    // We don't want to cycle forever.
    let mut visited = HashSet::new();

    // A graph of relations to traverse.
    let mut next_relations = Vec::new();

    // Gather all paths from this model to any other model, skipping
    // cyclical relations
    let mut paths = Vec::new();

    // Add all relations from current model to the graph vector. At this point
    // we only care about multiple paths from this model to any other model. We
    // handle paths that cross, but are not started from here, when calling the
    // function from corresponding models.

    let relation_fields = parent_model
        .relation_fields()
        .filter(|field| field.is_singular())
        .filter(|field| {
            let related_field = datamodel.find_related_field_bang(field).1;

            let on_update = field
                .relation_info
                .on_update
                .or(related_field.relation_info.on_update)
                .unwrap_or_else(|| field.default_on_update_action());

            let on_delete = field
                .relation_info
                .on_delete
                .or(related_field.relation_info.on_delete)
                .unwrap_or_else(|| field.default_on_delete_action());

            on_update.triggers_modification() || on_delete.triggers_modification()
        });

    for field in relation_fields {
        next_relations.push((
            parent_model,
            field,
            Rc::new(VisitedRelation::root(parent_model.name(), field.name())),
        ));
    }

    while let Some((model, field, visited_relations)) = next_relations.pop() {
        let related_field = datamodel.find_related_field_bang(field).1;
        let related_model = datamodel.find_model(&field.relation_info.to).unwrap();

        visited.insert((model.name(), field.name()));

        // Following a directed graph, we don't need to go through back-relations.
        if field.is_list() {
            continue;
        }

        let on_update = field
            .relation_info
            .on_update
            .or(related_field.relation_info.on_update)
            .unwrap_or_else(|| field.default_on_update_action());

        let on_delete = field
            .relation_info
            .on_delete
            .or(related_field.relation_info.on_delete)
            .unwrap_or_else(|| field.default_on_delete_action());

        // The path matters only if any of its actions modifies the children.
        if on_delete.triggers_modification() || on_update.triggers_modification() {
            // Self-relations are detected elsewhere.
            if model.name() == related_model.name() {
                continue;
            }

            // Cycles are detected elsewhere.
            if related_model.name() == parent_model.name() {
                continue;
            }

            // If the related model does not have any paths to other models, we
            // traversed the whole path, storing it to the collection for later
            // inspection.
            if !related_model.relation_fields().any(|f| f.is_singular()) {
                paths.push(visited_relations.link_model(related_model.name()));

                // We can, again, visit the same model/field combo when
                // traversing a different path.
                visited.clear();

                continue;
            }

            // Traversing all other relations from the next model.
            related_model
                .relation_fields()
                .filter(|f| f.is_singular())
                .filter(|f| !visited.contains(&(related_model.name(), f.name())))
                .for_each(|related_field| {
                    next_relations.push((
                        related_model,
                        related_field,
                        Rc::new(visited_relations.link_next(related_model.name(), related_field.name())),
                    ));
                });
        }
    }

    // Gather all paths from a relation field to all the models seen from that
    // field.
    let mut seen: HashMap<&str, HashSet<&str>> = HashMap::new();
    for path in paths.iter() {
        let mut iter = path.iter();

        let seen_models = match iter.next() {
            Some((_, Some(field_name))) => seen.entry(field_name).or_default(),
            _ => continue,
        };

        for (model_name, _) in iter {
            seen_models.insert(model_name);
        }
    }

    // Compact the model nameas that can be reached from the given relation
    // field, but also from any other relation field in the same model.
    let mut reachable: BTreeSet<&str> = BTreeSet::new();

    if let Some(from_parent) = seen.remove(parent_field.name().as_str()) {
        for (_, from_other) in seen.into_iter() {
            reachable.extend(from_parent.intersection(&from_other));
        }
    }

    let models = reachable.iter().map(|model| format!("`{}`", model)).join(", ");

    match reachable.len() {
        0 => (),
        1 => {
            let msg = format!(
                "When any of the records in model {} is updated or deleted, the referential actions on the relations cascade to model `{}` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`.", models, parent_model.name());

            errors.push_error(error_with_default_values(parent_field, &msg, span));
        }
        _ => {
            let msg = format!(
                "When any of the records in models {} are updated or deleted, the referential actions on the relations cascade to model `{}` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`.", models, parent_model.name());

            errors.push_error(error_with_default_values(parent_field, &msg, span));
        }
    }
}

/// Find relations returning back to the parent model with cascading referential
/// actions.
pub(crate) fn detect_cycles(
    datamodel: &Datamodel,
    parent_model: &Model,
    parent_field: &RelationField,
    span: Span,
    errors: &mut Diagnostics,
) {
    // Keeps count on visited relations to iterate them only once.
    let mut visited = HashSet::new();

    // poor man's tail-recursion ;)
    let mut next_relations = vec![(
        parent_model,
        parent_field,
        Rc::new(VisitedRelation::root(parent_model.name(), parent_field.name())),
    )];

    while let Some((model, field, visited_relations)) = next_relations.pop() {
        // we expect to have both sides of the relation at this point...
        let related_field = datamodel.find_related_field_bang(field).1;
        let related_model = datamodel.find_model(&field.relation_info.to).unwrap();

        // we do not visit the relation field on the other side
        // after this run.
        visited.insert((model.name(), field.name()));
        visited.insert((related_model.name(), related_field.name()));

        // Cycle only happens from the `@relation` side.
        if field.is_list() {
            continue;
        }

        // we skipped many-to-many relations, so one of the sides either has
        // referential actions set, or we can take the default actions
        let on_update = field
            .relation_info
            .on_update
            .or(related_field.relation_info.on_update)
            .unwrap_or_else(|| field.default_on_update_action());

        let on_delete = field
            .relation_info
            .on_delete
            .or(related_field.relation_info.on_delete)
            .unwrap_or_else(|| field.default_on_delete_action());

        // a cycle has a meaning only if every relation in it triggers
        // modifications in the children
        if on_delete.triggers_modification() || on_update.triggers_modification() {
            if model.name() == related_model.name() {
                let msg = "A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes.";
                errors.push_error(error_with_default_values(parent_field, msg, span));
                return;
            }

            if related_model.name() == parent_model.name() {
                let msg = format!("Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: {}.", visited_relations);
                errors.push_error(error_with_default_values(parent_field, &msg, span));
                return;
            }

            // bozo tail-recursion continues
            for field in related_model.relation_fields() {
                if !visited.contains(&(related_model.name(), field.name())) {
                    next_relations.push((
                        related_model,
                        field,
                        Rc::new(visited_relations.link_next(related_model.name(), field.name())),
                    ));
                }
            }
        }
    }
}

fn error_with_default_values(parent_field: &RelationField, msg: &str, span: Span) -> DatamodelError {
    let on_delete = match parent_field.relation_info.on_delete {
        None if parent_field.default_on_delete_action().triggers_modification() => {
            Some(parent_field.default_on_delete_action())
        }
        _ => None,
    };

    let on_update = match parent_field.relation_info.on_update {
        None if parent_field.default_on_update_action().triggers_modification() => {
            Some(parent_field.default_on_update_action())
        }
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

    DatamodelError::new_attribute_validation_error(&msg, RELATION_ATTRIBUTE_NAME, span)
}
