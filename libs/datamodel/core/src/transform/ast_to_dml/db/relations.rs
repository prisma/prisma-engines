use crate::ast;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Relations {
    /// This contains only the relation fields actually present in the schema
    /// source text.
    pub(super) relation_fields: BTreeMap<(ast::ModelId, ast::FieldId), RelationField>,
}

pub(crate) struct RelationField {
    pub(crate) referenced_model: ast::ModelId,
}
