use super::{super::helpers::*, AttributeValidator};
use crate::diagnostics::DatamodelError;
use crate::Field::{RelationField, ScalarField};
use crate::Ignorable;
use crate::{ast, dml, Datamodel};

/// Prismas builtin `@ignore` attribute.
pub struct IgnoreAttributeValidator {}

const ATTRIBUTE_NAME: &str = "ignore";

impl AttributeValidator<dml::Model> for IgnoreAttributeValidator {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Model) -> Result<(), DatamodelError> {
        if obj.fields().any(|f| f.is_ignored()) {
            return self.new_attribute_validation_error(
                "Fields on an already ignored Model do not need an `@ignore` annotation.",
                args.span(),
            );
        }

        obj.is_ignored = true;
        Ok(())
    }

    fn serialize(&self, obj: &dml::Model, _datamodel: &Datamodel) -> Vec<ast::Attribute> {
        internal_serialize(obj)
    }
}

pub struct IgnoreAttributeValidatorForField {}
impl AttributeValidator<dml::Field> for IgnoreAttributeValidatorForField {
    fn attribute_name(&self) -> &str {
        ATTRIBUTE_NAME
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        match obj {
            ScalarField(sf) if matches!(sf.field_type, dml::FieldType::Unsupported(_)) => {
                self.new_attribute_validation_error("Fields of type `Unsupported` cannot take an `@ignore` attribute. They are already treated as ignored by the client due to their type.", args.span())
            }

            ScalarField(sf) => {
                sf.is_ignored = true;
                Ok(())
            }
            RelationField(rf) => {
                rf.is_ignored = true;
                Ok(())
            }
        }
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
