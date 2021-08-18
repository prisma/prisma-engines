use std::{collections::HashSet, fmt, rc::Rc};

use dml::{datamodel::Datamodel, field::RelationField, model::Model, traits::WithName};

use crate::{
    ast::Span,
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::validate::RELATION_ATTRIBUTE_NAME,
};

/// A linked list structure for visited relation paths.
#[derive(Debug)]
struct VisitedRelation<'a> {
    previous: Option<Rc<VisitedRelation<'a>>>,
    model_name: &'a str,
    field_name: &'a str,
}

impl<'a> VisitedRelation<'a> {
    /// Create a new root node, starting a new relation path.
    fn root(model_name: &'a str, field_name: &'a str) -> Self {
        Self {
            previous: None,
            model_name,
            field_name,
        }
    }

    /// Links a relation to the current path.
    fn link_next(self: &Rc<Self>, model_name: &'a str, field_name: &'a str) -> Self {
        Self {
            previous: Some(self.clone()),
            model_name,
            field_name,
        }
    }
}

impl<'a> fmt::Display for VisitedRelation<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut traversed_models = vec![format!("{}.{}", self.model_name, self.field_name)];
        let mut this = self;

        while let Some(next) = this.previous.as_ref() {
            traversed_models.push(format!("{}.{}", next.model_name, next.field_name));
            this = next;
        }

        traversed_models.reverse();

        write!(f, "{}", traversed_models.join(" → "))
    }
}

/// In certain databases, such as SQL Server, it is not allowd to create
/// multiple reference paths between two models, if referential actions would
/// cause modifications to the children objects.
///
/// We detect this early before letting database to give us a much more
/// cryptic error message.
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

        // skip many-to-many
        if field.is_list() && related_field.is_list() {
            continue;
        }

        // we skipped many-to-many relations, so one of the sides either has
        // referential actions set, or we can take the default actions
        let on_update = field
            .relation_info
            .on_update
            .or(related_field.relation_info.on_update)
            .unwrap_or_else(|| {
                if field.is_list() {
                    related_field.default_on_update_action()
                } else {
                    field.default_on_update_action()
                }
            });

        let on_delete = field
            .relation_info
            .on_delete
            .or(related_field.relation_info.on_delete)
            .unwrap_or_else(|| {
                if field.is_list() {
                    related_field.default_on_delete_action()
                } else {
                    field.default_on_delete_action()
                }
            });

        // a cycle has a meaning only if every relation in it triggers
        // modifications in the children
        if on_delete.triggers_modification() || on_update.triggers_modification() {
            let error_with_default_values = |msg: &str| {
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
                            "{} Implicit default `onDelete` and `onUpdate` values: `{}` and `{}`.",
                            msg, on_delete, on_update
                        )
                    }
                    (Some(on_delete), None) => {
                        format!("{} Implicit default `onDelete` value: `{}`.", msg, on_delete)
                    }
                    (None, Some(on_update)) => {
                        format!("{} Implicit default `onUpdate` value: `{}`.", msg, on_update)
                    }
                    (None, None) => msg.to_string(),
                };

                msg.push_str(" Read more at https://pris.ly/d/cyclic-referential-actions");

                DatamodelError::new_attribute_validation_error(&msg, RELATION_ATTRIBUTE_NAME, span)
            };

            if model.name() == related_model.name() {
                let msg = "A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes.";
                errors.push_error(error_with_default_values(msg));
                return;
            }

            if related_model.name() == parent_model.name() {
                let msg = format!("Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: {}.", visited_relations);
                errors.push_error(error_with_default_values(&msg));
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
