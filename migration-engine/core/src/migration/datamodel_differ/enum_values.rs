use super::AttributeDiffer;
use datamodel::ast;

pub(crate) struct EnumValueDiffer<'a> {
    pub(crate) previous: &'a ast::EnumValue,
    pub(crate) next: &'a ast::EnumValue,
}

impl<'a> EnumValueDiffer<'a> {
    pub(crate) fn attribute_pairs<'b>(&'b self) -> impl Iterator<Item = AttributeDiffer<'a>> + 'b {
        self.previous_attributes().filter_map(move |previous| {
            self.next_attributes()
                .find(|next| enum_value_attributes_match(previous, next))
                .map(|next| AttributeDiffer { previous, next })
        })
    }

    pub(crate) fn created_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.next_attributes().filter(move |next| {
            !self
                .previous_attributes()
                .any(|previous| enum_value_attributes_match(previous, next))
        })
    }

    pub(crate) fn deleted_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.previous_attributes().filter(move |previous| {
            !self
                .next_attributes()
                .any(|next| enum_value_attributes_match(previous, next))
        })
    }

    fn previous_attributes<'b>(&'b self) -> impl Iterator<Item = &'a ast::Attribute> + 'b {
        self.previous.attributes.iter()
    }

    fn next_attributes<'b>(&'b self) -> impl Iterator<Item = &'a ast::Attribute> + 'b {
        self.next.attributes.iter()
    }
}

fn enum_value_attributes_match(previous: &ast::Attribute, next: &ast::Attribute) -> bool {
    previous.name.name == next.name.name
}
