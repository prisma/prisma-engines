use super::attributes::{attributes_match, AttributeDiffer};
use datamodel::ast;

/// Implements the logic to diff a pair of [Field ASTs](/datamodel/ast/struct.Field.html).
#[derive(Debug)]
pub(crate) struct FieldDiffer<'a> {
    pub(crate) previous: &'a ast::Field,
    pub(crate) next: &'a ast::Field,
}

impl<'a> FieldDiffer<'a> {
    /// Iterator over the attributes present in `next` but not in `previous`.
    pub(crate) fn created_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.next_attributes().filter(move |next_attribute| {
            self.previous_attributes()
                .find(|previous_attribute| attributes_match(previous_attribute, next_attribute))
                .is_none()
        })
    }

    /// Iterator over the attributes present in `previous` but not in `next`.
    pub(crate) fn deleted_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.previous_attributes().filter(move |previous_attribute| {
            self.next_attributes()
                .find(|next_attribute| attributes_match(previous_attribute, next_attribute))
                .is_none()
        })
    }

    pub(crate) fn attribute_pairs(&self) -> impl Iterator<Item = AttributeDiffer<'_>> {
        self.previous_attributes().filter_map(move |previous_attribute| {
            self.next_attributes()
                .find(|next_attribute| attributes_match(previous_attribute, next_attribute))
                .map(|next_attribute| AttributeDiffer {
                    previous: previous_attribute,
                    next: next_attribute,
                })
        })
    }

    fn previous_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.previous.attributes.iter()
    }

    fn next_attributes(&self) -> impl Iterator<Item = &ast::Attribute> {
        self.next.attributes.iter()
    }
}
