use super::FieldBuilder;
use crate::{CompositeType, CompositeTypeRef, InternalDataModelWeakRef};
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct CompositeTypeBuilder {
    pub name: String,
    pub fields: Vec<FieldBuilder>,
}

/// Processes all composites as a unit due to potential cycles and references.
pub(crate) fn build_composites(
    builders: Vec<CompositeTypeBuilder>,
    internal_data_model: InternalDataModelWeakRef,
) -> Vec<CompositeTypeRef> {
    let mut composites = Vec::with_capacity(builders.len());
    let mut fields = std::collections::HashMap::new();

    // First pass: Builder the references (arcs) and store the fields for processing.
    for builder in builders {
        composites.push(Arc::new(CompositeType {
            name: builder.name.clone(),
            internal_data_model: internal_data_model.clone(),
            fields: OnceCell::new(),
        }));

        fields.insert(builder.name, builder.fields);
    }

    // Second pass: Build fields. Unwraps are safe as the composite must exist from the first pass.
    for (name, fields) in fields {
        let composite = composites.iter().find(|c| c.name == name).unwrap();
        let fields = fields
            .into_iter()
            .map(|builder| builder.build(Arc::downgrade(composite).into(), &composites))
            .collect();

        // Unwrap is safe - the fields have been empty so far.
        composite.fields.set(fields).unwrap();
    }

    composites
}
