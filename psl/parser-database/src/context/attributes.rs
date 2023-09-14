use super::*;
use crate::interner::StringId;

#[derive(Default, Debug)]
pub(super) struct AttributesValidationState {
    /// The attributes list being validated.
    pub(super) attributes: Option<crate::AttributeContainer>,
    pub(super) unused_attributes: HashSet<ast::AttributeId>, // the _remaining_ attributes

    /// The attribute being validated.
    pub(super) attribute: Option<crate::AttributeId>,
    pub(super) args: HashMap<Option<StringId>, usize>, // the _remaining_ arguments of `attribute`
}

impl AttributesValidationState {
    pub(super) fn set_attributes(&mut self, attributes: crate::AttributeContainer, ast: &ast::SchemaAst) {
        let attribute_ids = (0..ast[attributes].len()).map(|idx| ast::AttributeId::new_in_container(attributes, idx));
        self.unused_attributes.clear();
        self.unused_attributes.extend(attribute_ids);

        self.attributes = Some(attributes);
    }
}
