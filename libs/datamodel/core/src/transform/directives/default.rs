use super::{super::helpers::*, DirectiveValidator};
use crate::error::DatamodelError;
use crate::{ast, dml, DefaultValue, ValueGenerator};
use prisma_value::PrismaValue;

/// Prismas builtin `@default` directive.
pub struct DefaultDirectiveValidator {}

impl DirectiveValidator<dml::Field> for DefaultDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"default"
    }

    fn validate_and_apply(&self, args: &mut Arguments, field: &mut dml::Field) -> Result<(), DatamodelError> {
        if let dml::Field::RelationField(_) = field {
            return self.new_directive_validation_error("Cannot set a default value on a relation field.", args.span());
        } else if let dml::Field::ScalarField(sf) = field {
            // If we allow list default values, we need to adjust the types below properly for that case.
            if sf.arity == dml::FieldArity::List {
                return self.new_directive_validation_error("Cannot set a default value on list field.", args.span());
            }

            if let dml::FieldType::Base(scalar_type, _) = sf.field_type {
                let dv = args
                    .default_arg("value")?
                    .as_default_value_for_scalar_type(scalar_type)
                    .map_err(|e| self.wrap_in_directive_validation_error(&e))?;

                sf.default_value = Some(dv);
            } else if let dml::FieldType::NativeType(scalar_type, native_type) = sf.field_type.clone() {
                let dv = args
                    .default_arg("value")?
                    .as_default_value_for_scalar_type(scalar_type)
                    .map_err(|e| self.wrap_in_directive_validation_error(&e))?;

                sf.default_value = Some(dv);

                if native_type.name == "Serial" {
                    // assuming this must be a Postgres native type
                    if let Some(arg) = sf.default_value.clone() {
                        match arg {
                            DefaultValue::Expression(o) => {
                                if o.name == "autoincrement" {
                                    return Err(self.wrap_in_directive_validation_error(&DatamodelError::new_connector_error(
                                        "The native type serial translates to an Integer column with an auto-incrementing counter as default. The field attribute @default(autoincrement()) translates to the serial type underneath. Please remove one of the two attributes.",
                                    args.span())));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            } else if let dml::FieldType::Enum(_) = sf.field_type {
                let default_arg = args.default_arg("value")?;

                match default_arg.as_constant_literal() {
                    Ok(value) => sf.default_value = Some(dml::DefaultValue::Single(PrismaValue::Enum(value))),
                    Err(err) => {
                        let generator = default_arg.as_value_generator()?;
                        if generator == ValueGenerator::new_dbgenerated() {
                            sf.default_value = Some(dml::DefaultValue::Expression(generator));
                        } else {
                            return Err(self.wrap_in_directive_validation_error(&err));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn serialize(
        &self,
        field: &dml::Field,
        _datamodel: &dml::Datamodel,
    ) -> Result<Vec<ast::Directive>, DatamodelError> {
        if let Some(default_value) = field.default_value() {
            return Ok(vec![ast::Directive::new(
                self.directive_name(),
                vec![ast::Argument::new("", lower_default_value(default_value.clone()))],
            )]);
        }

        Ok(vec![])
    }
}

pub fn lower_default_value(dv: dml::DefaultValue) -> ast::Expression {
    match dv {
        dml::DefaultValue::Single(v) => lower_prisma_value(&v),
        dml::DefaultValue::Expression(e) => {
            let exprs = e.args.iter().map(lower_prisma_value).collect();
            ast::Expression::Function(e.name, exprs, ast::Span::empty())
        }
    }
}

pub fn lower_prisma_value(pv: &PrismaValue) -> ast::Expression {
    match pv {
        PrismaValue::Boolean(true) => ast::Expression::BooleanValue(String::from("true"), ast::Span::empty()),
        PrismaValue::Boolean(false) => ast::Expression::BooleanValue(String::from("false"), ast::Span::empty()),
        PrismaValue::String(value) => ast::Expression::StringValue(value.clone(), ast::Span::empty()),
        PrismaValue::Enum(value) => ast::Expression::ConstantValue(value.clone(), ast::Span::empty()),
        PrismaValue::DateTime(value) => ast::Expression::StringValue(value.to_rfc3339(), ast::Span::empty()),
        PrismaValue::Float(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
        PrismaValue::Int(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
        PrismaValue::Null => ast::Expression::ConstantValue("null".to_string(), ast::Span::empty()),
        PrismaValue::Uuid(val) => ast::Expression::StringValue(val.to_string(), ast::Span::empty()),
        PrismaValue::Json(val) => ast::Expression::StringValue(val.to_string(), ast::Span::empty()),
        PrismaValue::List(vec) => ast::Expression::Array(
            vec.iter().map(|pv| lower_prisma_value(pv)).collect(),
            ast::Span::empty(),
        ),
    }
}
