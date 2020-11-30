use super::{super::helpers::*, AttributeValidator};
use crate::diagnostics::DatamodelError;
use crate::{ast, dml};

/// Prismas builtin `@updatedAt` attribute.
pub struct UpdatedAtAttributeValidator {}

impl AttributeValidator<dml::Field> for UpdatedAtAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        &"updatedAt"
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        if let dml::Field::ScalarField(sf) = obj {
            if sf.field_type.scalar_type() == Some(dml::ScalarType::DateTime) {
                if sf.arity == dml::FieldArity::List {
                    return self.new_attribute_validation_error(
                        "Fields that are marked with @updatedAt can not be lists.",
                        args.span(),
                    );
                }

                sf.is_updated_at = true;

                return Ok(());
            }
        }
        self.new_attribute_validation_error(
            "Fields that are marked with @updatedAt must be of type DateTime.",
            args.span(),
        )
    }

    fn serialize(&self, field: &dml::Field, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if field.is_updated_at() {
            vec![ast::Attribute::new(self.attribute_name(), Vec::new())]
        } else {
            vec![]
        }
    }
}
