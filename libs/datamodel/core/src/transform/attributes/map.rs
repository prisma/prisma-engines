use super::{super::helpers::*, AttributeValidator};
use crate::ast::{Attribute, Span};
use crate::diagnostics::DatamodelError;
use crate::{ast, dml, Datamodel, WithDatabaseName};

/// Prismas builtin `@map` attribute.
pub struct MapAttributeValidator {}

const ATTRIBUTE_NAME: &str = "map";

impl AttributeValidator<dml::Model> for MapAttributeValidator {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::Model, _datamodel: &Datamodel) -> Result<Vec<Attribute>, DatamodelError> {
        internal_serialize(obj)
    }
}

pub struct MapAttributeValidatorForField {}
impl AttributeValidator<dml::Field> for MapAttributeValidatorForField {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        if obj.is_relation() {
            return self.new_attribute_validation_error(
                &format!(
                    "The attribute `@{}` can not be used on relation fields.",
                    ATTRIBUTE_NAME
                ),
                args.span(),
            );
        }
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::Field, _datamodel: &Datamodel) -> Result<Vec<Attribute>, DatamodelError> {
        internal_serialize(obj)
    }
}

impl AttributeValidator<dml::Enum> for MapAttributeValidator {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Enum) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::Enum, _datamodel: &Datamodel) -> Result<Vec<Attribute>, DatamodelError> {
        internal_serialize(obj)
    }
}

impl AttributeValidator<dml::EnumValue> for MapAttributeValidator {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::EnumValue) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::EnumValue, _datamodel: &Datamodel) -> Result<Vec<Attribute>, DatamodelError> {
        internal_serialize(obj)
    }
}

fn internal_validate_and_apply(args: &mut Arguments, obj: &mut dyn WithDatabaseName) -> Result<(), DatamodelError> {
    let db_name = args.default_arg("name")?.as_str().map_err(|err| {
        DatamodelError::new_attribute_validation_error(&format!("{}", err), ATTRIBUTE_NAME, err.span())
    })?;
    obj.set_database_name(Some(db_name));
    Ok(())
}

fn internal_serialize(obj: &dyn WithDatabaseName) -> Result<Vec<ast::Attribute>, DatamodelError> {
    match obj.database_name() {
        Some(db_name) => Ok(vec![ast::Attribute::new(
            ATTRIBUTE_NAME,
            vec![ast::Argument::new_unnamed(ast::Expression::StringValue(
                String::from(db_name),
                Span::empty(),
            ))],
        )]),
        None => Ok(vec![]),
    }
}
