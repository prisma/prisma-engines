use crate::{CompositeType, CompositeTypeRef, InternalDataModelWeakRef};
use psl::schema_ast::ast;
use std::sync::Arc;

#[derive(Debug)]
pub struct CompositeTypeBuilder {
    pub id: ast::CompositeTypeId,
    pub name: String,
}

/// Processes all composites as a unit due to potential cycles and references.
pub(crate) fn build_composites(
    builders: Vec<CompositeTypeBuilder>,
    internal_data_model: InternalDataModelWeakRef,
) -> Vec<CompositeTypeRef> {
    builders
        .into_iter()
        .map(|builder| {
            Arc::new(CompositeType {
                id: builder.id,
                name: builder.name,
                internal_data_model: internal_data_model.clone(),
            })
        })
        .collect()
}
