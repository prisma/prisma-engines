use super::ScalarFieldType;
use crate::{
    ast::{FieldId, SchemaAst, TopId},
    diagnostics::{DatamodelError, Diagnostics},
};
use itertools::Itertools;
use std::collections::{BTreeMap, BTreeSet, HashMap};

type AnnotationId = u32;

#[derive(Debug, Default)]
pub(super) struct Types {
    pub(super) type_aliases: BTreeMap<TopId, ScalarFieldType>,
    pub(super) scalar_fields: BTreeMap<(TopId, FieldId), ScalarFieldType>,
    annotations: Vec<ModelAnnotation>,
    // Storage for annotation fields, i.e. the fields referenced in `@(@)index`,
    // `@(@)unique` and `@(@)id`. The type should be understood as (model_id,
    // annotation_id, order_key_in_annotation, field_id)
    annotation_fields: BTreeSet<(TopId, AnnotationId, u16, FieldId)>,
}

impl Types {
    /// Detect self-referencing type aliases (possibly indirectly).
    pub(super) fn detect_alias_cycles(&self, ast: &SchemaAst, diagnostics: &mut Diagnostics) {
        let mut path = Vec::new();
        // We accumulate the errors here because we want to sort them at the end.
        let mut errors: Vec<(TopId, DatamodelError)> = Vec::new();

        for (top_id, ty) in &self.type_aliases {
            let mut current = (*top_id, ty);
            path.clear();

            // Follow the chain.
            while let ScalarFieldType::Alias(next_alias_id) = current.1 {
                path.push(current.0);
                let next_alias = ast[*next_alias_id].unwrap_type_alias();
                // Detect a cycle where next type is also the root. In that
                // case, we want to report an error.
                if path.len() > 1 && &path[0] == next_alias_id {
                    errors.push((
                        *top_id,
                        DatamodelError::new_validation_error(
                            &format!(
                                "Recursive type definitions are not allowed. Recursive path was: {} -> {}.",
                                path.iter()
                                    .map(|id| &ast[*id].unwrap_type_alias().name.name)
                                    .join(" -> "),
                                &next_alias.name.name,
                            ),
                            next_alias.field_type.span(),
                        ),
                    ));
                    break;
                }

                // Detect a cycle anywhere else in the chain of native
                // types. In that case, the error will be reported somewhere
                // else, and we can just abort.
                if path.contains(next_alias_id) {
                    break;
                }

                match self.type_aliases.get(next_alias_id) {
                    Some(next_alias_type) => {
                        current = (*next_alias_id, next_alias_type);
                    }
                    // A missing alias at this point means that there was an
                    // error resolving the type of the next alias. We should
                    // stop validation here.
                    None => break,
                }
            }
        }

        errors.sort_by_key(|(id, _err)| *id);
        for (_, error) in errors {
            diagnostics.push_error(error);
        }
    }
}

#[derive(Debug)]
pub(crate) enum ModelAnnotation {
    Id,
    Index,
    Unique,
    Ignore,
}
