use super::AttributeValidator;
use crate::{ast, dml};
// BTreeMap has a strictly defined order.
// That's important since rendering depends on that order.
use std::collections::BTreeMap;

/// Struct which holds a list of attribute validators and automatically
/// picks the right one for each attribute in the given object.
pub struct AttributeListValidator<T> {
    known_attributes: BTreeMap<&'static str, Box<dyn AttributeValidator<T>>>,
}

impl<T: 'static> AttributeListValidator<T> {
    pub fn new() -> Self {
        AttributeListValidator {
            known_attributes: BTreeMap::new(),
        }
    }

    /// Adds an attribute validator.
    pub fn add(&mut self, validator: Box<dyn AttributeValidator<T>>) {
        let name = validator.attribute_name();

        if self.known_attributes.insert(name, validator).is_some() {
            panic!("Duplicate attribute definition: {:?}", name);
        }
    }

    pub fn serialize(&self, t: &T, datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        self.known_attributes
            .values()
            .map(|attribute| attribute.serialize(t, datamodel))
            .flatten()
            .collect()
    }
}
