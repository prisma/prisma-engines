use super::{super::helpers::*, AttributeValidator};
use crate::ast::Span;
use crate::diagnostics::DatamodelError;
use crate::{ast, dml, Datamodel, WithDatabaseName};

/// Prismas builtin `@map` attribute.
pub struct MapAttributeValidator {}

const ATTRIBUTE_NAME: &str = "map";

impl AttributeValidator<dml::Model> for MapAttributeValidator {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::Model, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

pub struct MapAttributeValidatorForField {}
impl AttributeValidator<dml::Field> for MapAttributeValidatorForField {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Field) -> Result<(), DatamodelError> {
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

    fn serialize(&self, obj: &dml::Field, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

impl AttributeValidator<dml::Enum> for MapAttributeValidator {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Enum) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::Enum, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

impl AttributeValidator<dml::EnumValue> for MapAttributeValidator {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::EnumValue) -> Result<(), DatamodelError> {
        internal_validate_and_apply(args, obj)
    }

    fn serialize(&self, obj: &dml::EnumValue, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

fn internal_validate_and_apply(args: &mut Arguments<'_>, obj: &mut dyn WithDatabaseName) -> Result<(), DatamodelError> {
    let name_arg = args.default_arg("name")?;
    let db_name = name_arg.as_str().map_err(|err| {
        DatamodelError::new_attribute_validation_error(&format!("{}", err), ATTRIBUTE_NAME, err.span())
    })?;

    obj.set_database_name(Some(db_name.to_owned()));

    Ok(())
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
