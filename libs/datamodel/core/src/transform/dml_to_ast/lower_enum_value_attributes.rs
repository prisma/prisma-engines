use super::*;
use crate::{ast, dml};

/// Internal: Lowers an enum's attributes.
pub(super) fn lower_enum_value_attributes(enum_value: &dml::EnumValue) -> Vec<ast::Attribute> {
    let mut attributes = vec![];

    push_model_index_map_arg(enum_value, &mut attributes);

    attributes
}
