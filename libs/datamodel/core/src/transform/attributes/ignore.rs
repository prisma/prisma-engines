use super::AttributeValidator;
use crate::Ignorable;
use crate::{ast, dml, Datamodel};

/// Prismas builtin `@ignore` attribute.
pub struct IgnoreAttributeValidator {}

const ATTRIBUTE_NAME: &str = "ignore";

impl AttributeValidator<dml::Model> for IgnoreAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        ATTRIBUTE_NAME
    }

    fn serialize(&self, obj: &dml::Model, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

pub struct IgnoreAttributeValidatorForField {}
impl AttributeValidator<dml::Field> for IgnoreAttributeValidatorForField {
    fn attribute_name(&self) -> &'static str {
        ATTRIBUTE_NAME
    }

    fn serialize(&self, obj: &dml::Field, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

fn internal_serialize(obj: &dyn Ignorable) -> Vec<ast::Attribute> {
    match obj.is_ignored() {
        true => vec![ast::Attribute::new(ATTRIBUTE_NAME, vec![])],
        false => vec![],
    }
}
