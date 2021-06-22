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
    pub(super) type_aliases: HashMap<TopId, ScalarFieldType>,
    pub(super) scalar_fields: BTreeMap<(TopId, FieldId), ScalarFieldType>,
    annotations: Vec<ModelAnnotation>,
    // Storage for annotation fields, i.e. the fields referenced in `@(@)index`,
    // `@(@)unique` and `@(@)id`. The type should be understood as (model_id,
    // annotation_id, sort_key_, field_id), where the sort key is the index of
    // the field in the annotation.
    annotation_fields: BTreeSet<(TopId, AnnotationId, u16, FieldId)>,
}

impl Types {
    /// Detect self-referencing type aliases, possibly indirectly. We loop
    /// through each type alias in the schema. If it references another type
    /// alias — which may in turn reference another type alias —, we check that
    /// it is not self-referencing. If a type alias ends up transitively
    /// referencing itself, we create an error diagnostic.
    pub(super) fn detect_alias_cycles(&self, ast: &SchemaAst, diagnostics: &mut Diagnostics) {
        // The IDs of the type aliases we traversed to get to the current type alias.
        let mut path = Vec::new();
        // We accumulate the errors here because we want to sort them at the end.
        let mut errors: Vec<(TopId, DatamodelError)> = Vec::new();

        for (top_id, ty) in &self.type_aliases {
            // Loop variable. This is the "tip" of the sequence of type aliases.
            let mut current = (*top_id, ty);
            path.clear();

            // Follow the chain of type aliases referencing other type aliases.
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

                // We detect a cycle anywhere else in the chain of type aliases.
                // In that case, the error will be reported somewhere else, and
                // we can just move on from this alias.
                if path.contains(next_alias_id) {
                    break;
                }

                match self.type_aliases.get(next_alias_id) {
                    Some(next_alias_type) => {
                        current = (*next_alias_id, next_alias_type);
                    }
                    // A missing alias at this point means that there was an
                    // error resolving the type of the next alias. We stop
                    // validation here.
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
