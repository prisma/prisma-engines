use super::*;
use crate::{ast, dml};

/// Internal: Lowers an enum's attributes.
pub(super) fn lower_enum_attributes(enm: &dml::Enum) -> Vec<ast::Attribute> {
    let mut attributes = vec![];

    push_model_index_map_arg(enm, &mut attributes);

    attributes
}
