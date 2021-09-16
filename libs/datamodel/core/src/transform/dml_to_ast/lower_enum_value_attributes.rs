use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self},
    dml,
};

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers an enum's attributes.
    pub(crate) fn lower_enum_value_attributes(&self, enum_value: &dml::EnumValue) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        <LowerDmlToAst<'a>>::push_map_attribute(enum_value, &mut attributes);

        attributes
    }
}
