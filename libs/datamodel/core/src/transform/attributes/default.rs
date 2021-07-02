use super::AttributeValidator;
use crate::{ast, dml};
use prisma_value::PrismaValue;

/// Prismas builtin `@default` attribute.
pub struct DefaultAttributeValidator;

impl AttributeValidator<dml::Field> for DefaultAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        "default"
    }

    fn serialize(&self, field: &dml::Field, _datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if let Some(default_value) = field.default_value() {
            return vec![ast::Attribute::new(
                self.attribute_name(),
                vec![ast::Argument::new("", lower_default_value(default_value.clone()))],
            )];
        }

        vec![]
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
        PrismaValue::BigInt(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
        PrismaValue::Null => ast::Expression::ConstantValue("null".to_string(), ast::Span::empty()),
        PrismaValue::Uuid(val) => ast::Expression::StringValue(val.to_string(), ast::Span::empty()),
        PrismaValue::Json(val) => ast::Expression::StringValue(val.to_string(), ast::Span::empty()),
        PrismaValue::List(vec) => ast::Expression::Array(
            vec.iter().map(|pv| lower_prisma_value(pv)).collect(),
            ast::Span::empty(),
        ),
        PrismaValue::Xml(val) => ast::Expression::StringValue(val.to_string(), ast::Span::empty()),
        PrismaValue::Bytes(b) => ast::Expression::StringValue(prisma_value::encode_bytes(b), ast::Span::empty()),
    }
}
