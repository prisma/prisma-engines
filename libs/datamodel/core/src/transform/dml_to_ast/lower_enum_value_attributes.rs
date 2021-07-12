use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self, Span},
    dml, WithDatabaseName,
};

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers an enum's attributes.
    pub(crate) fn lower_enum_value_attributes(&self, enum_value: &dml::EnumValue) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        if let Some(db_name) = enum_value.database_name() {
            attributes.push(ast::Attribute::new(
                "map",
                vec![ast::Argument::new_unnamed(ast::Expression::StringValue(
                    String::from(db_name),
                    Span::empty(),
                ))],
            ));
        }

        attributes
    }
}
