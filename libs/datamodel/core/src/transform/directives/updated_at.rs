use super::{super::helpers::*, DirectiveValidator};
use crate::error::DatamodelError;
use crate::{ast, dml};

/// Prismas builtin `@updatedAt` directive.
pub struct UpdatedAtDirectiveValidator {}

impl DirectiveValidator<dml::Field> for UpdatedAtDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"updatedAt"
    }

    fn validate_and_apply(&self, args: &mut Arguments, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        if let dml::Field::ScalarField(sf) = obj {
            if sf.field_type.scalar_type() == Some(dml::ScalarType::DateTime) {
                if sf.arity == dml::FieldArity::List {
                    return self.new_directive_validation_error(
                        "Fields that are marked with @updatedAt can not be lists.",
                        args.span(),
                    );
                }

                sf.is_updated_at = true;

                return Ok(());
            }
        }
        self.new_directive_validation_error(
            "Fields that are marked with @updatedAt must be of type DateTime.",
            args.span(),
        )
    }

    fn serialize(
        &self,
        field: &dml::Field,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        if field.is_updated_at() {
            Ok(vec![ast::Directive::new(self.directive_name(), Vec::new())])
        } else {
            Ok(vec![])
        }
    }
}
