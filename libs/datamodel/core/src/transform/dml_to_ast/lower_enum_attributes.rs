use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self},
    dml,
};

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers an enum's attributes.
    pub(crate) fn lower_enum_attributes(&self, enm: &dml::Enum) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        <LowerDmlToAst<'a>>::push_model_index_map_arg(enm, &mut attributes);

        attributes
    }
}
