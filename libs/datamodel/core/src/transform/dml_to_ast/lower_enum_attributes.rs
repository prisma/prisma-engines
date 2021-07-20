use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self},
    dml,
};

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers an enum's attributes.
    pub(crate) fn lower_enum_attributes(&self, enm: &dml::Enum) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        <LowerDmlToAst<'a>>::push_map_attribute(enm, &mut attributes);

        attributes
    }
}
