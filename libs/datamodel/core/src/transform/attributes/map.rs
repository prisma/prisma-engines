use super::AttributeValidator;
use crate::ast::Span;
use crate::{ast, dml, Datamodel, WithDatabaseName};

/// Prismas builtin `@map` attribute.
pub struct MapAttributeValidator;

const ATTRIBUTE_NAME: &str = "map";

impl AttributeValidator<dml::Model> for MapAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        ATTRIBUTE_NAME
    }

    fn serialize(&self, obj: &dml::Model, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

pub struct MapAttributeValidatorForField {}
impl AttributeValidator<dml::Field> for MapAttributeValidatorForField {
    fn attribute_name(&self) -> &'static str {
        ATTRIBUTE_NAME
    }

    fn serialize(&self, obj: &dml::Field, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

impl AttributeValidator<dml::Enum> for MapAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        ATTRIBUTE_NAME
    }

    fn serialize(&self, obj: &dml::Enum, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

impl AttributeValidator<dml::EnumValue> for MapAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        ATTRIBUTE_NAME
    }

    fn serialize(&self, obj: &dml::EnumValue, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

fn internal_serialize(obj: &dyn WithDatabaseName) -> Vec<ast::Attribute> {
    match obj.database_name() {
        Some(db_name) => vec![ast::Attribute::new(
            ATTRIBUTE_NAME,
            vec![ast::Argument::new_unnamed(ast::Expression::StringValue(
                String::from(db_name),
                Span::empty(),
            ))],
        )],
        None => vec![],
    }
}
