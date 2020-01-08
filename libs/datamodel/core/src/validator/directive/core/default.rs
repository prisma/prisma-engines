use crate::common::value::ValueValidator;
use crate::error::DatamodelError;
use crate::validator::directive::{Args, DirectiveValidator};
use crate::{ast, dml, DefaultValue, ValueGenerator};
use std::convert::TryInto;

/// Prismas builtin `@default` directive.
pub struct DefaultDirectiveValidator {}

impl DirectiveValidator<dml::Field> for DefaultDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"default"
    }

    fn validate_and_apply(&self, args: &mut Args, field: &mut dml::Field) -> Result<(), DatamodelError> {
        // If we allow list default values, we need to adjust the types below properly for that case.
        if field.arity == dml::FieldArity::List {
            return self.error("Cannot set a default value on list field.", args.span());
        }

        if let dml::FieldType::Base(scalar_type) = field.field_type {
            let arg = args.default_arg_new("value")?;
            let dv = match &arg.value {
                ast::Expression::Function(name, _, _) => {
                    DefaultValue::Expression(ValueGenerator::new(name.to_string(), vec![])?)
                }
                _ => {
                    let x = ValueValidator::new(&arg.value)?.as_type(scalar_type);
                    let x = x.map_err(|e| self.parser_error(&e))?;
                    DefaultValue::Single(x)
                }
            };

            if dv.get_type() != scalar_type {
                return self.error(
                    &format!(
                        "Default value type {:?} doesn't match expected type {:?}.",
                        dv.get_type(),
                        scalar_type
                    ),
                    args.span(),
                );
            }
            field.default_value = Some(dv);
        } else if let dml::FieldType::Enum(_) = &field.field_type {
            match args.default_arg("value")?.as_constant_literal() {
                // TODO: We should also check if this value is a valid enum value.
                Ok(value) => field.default_value = Some(dml::ScalarValue::ConstantLiteral(value).try_into()?),
                Err(err) => return Err(self.parser_error(&err)),
            }
        } else {
            return self.error("Cannot set a default value on a relation field.", args.span());
        }

        Ok(())
    }

    fn serialize(
        &self,
        field: &dml::Field,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        if let Some(default_value) = &field.default_value {
            return Ok(vec![ast::Directive::new(
                self.directive_name(),
                vec![ast::Argument::new("", default_value.clone().into())],
            )]);
        }

        Ok(vec![])
    }
}
