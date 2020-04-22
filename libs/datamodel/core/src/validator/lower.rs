use super::DirectiveBox;
use crate::error::ErrorCollection;
use crate::{ast, dml};
use prisma_value::PrismaValue;

pub struct LowerDmlToAst {
    directives: DirectiveBox,
}

impl LowerDmlToAst {
    /// Creates a new instance, with all builtin directives registered.
    pub fn new() -> Self {
        Self {
            directives: DirectiveBox::new(),
        }
    }

    pub fn lower(&self, datamodel: &dml::Datamodel) -> Result<ast::SchemaAst, ErrorCollection> {
        let mut tops: Vec<ast::Top> = Vec::new();
        let mut errors = ErrorCollection::new();

        for model in datamodel.models() {
            if !model.is_generated {
                match self.lower_model(model, datamodel) {
                    Ok(res) => tops.push(ast::Top::Model(res)),
                    Err(mut err) => errors.append(&mut err),
                }
            }
        }

        for enm in datamodel.enums() {
            match self.lower_enum(enm, datamodel) {
                Ok(res) => tops.push(ast::Top::Enum(res)),
                Err(mut err) => errors.append(&mut err),
            }
        }

        Ok(ast::SchemaAst { tops: tops })
    }

    pub fn lower_model(&self, model: &dml::Model, datamodel: &dml::Datamodel) -> Result<ast::Model, ErrorCollection> {
        let mut errors = ErrorCollection::new();
        let mut fields: Vec<ast::Field> = Vec::new();

        for field in model.fields() {
            match self.lower_field(field, datamodel) {
                Ok(ast_field) => fields.push(ast_field),
                Err(mut err) => errors.append(&mut err),
            };
        }

        if errors.has_errors() {
            return Err(errors);
        }

        Ok(ast::Model {
            name: ast::Identifier::new(&model.name),
            fields,
            directives: self.directives.model.serialize(model, datamodel)?,
            documentation: model.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
            commented_out: model.is_commented_out,
        })
    }

    fn lower_enum(&self, enm: &dml::Enum, datamodel: &dml::Datamodel) -> Result<ast::Enum, ErrorCollection> {
        Ok(ast::Enum {
            name: ast::Identifier::new(&enm.name),
            values: enm
                .values
                .iter()
                .map(|v| ast::EnumValue {
                    name: ast::Identifier::new(&v.name),
                    directives: self.directives.enm_value.serialize(v, datamodel).unwrap(),
                    span: ast::Span::empty(),
                    commented_out: v.commented_out,
                })
                .collect(),
            directives: self.directives.enm.serialize(enm, datamodel)?,
            documentation: enm.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        })
    }

    fn lower_field(&self, field: &dml::Field, datamodel: &dml::Datamodel) -> Result<ast::Field, ErrorCollection> {
        Ok(ast::Field {
            name: ast::Identifier::new(&field.name),
            arity: self.lower_field_arity(field.arity),
            default_value: field.default_value.clone().map(|dv| Self::lower_default_value(dv)),
            directives: self.directives.field.serialize(field, datamodel)?,
            field_type: self.lower_type(&field.field_type),
            documentation: field.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
            is_commented_out: field.is_commented_out,
        })
    }

    /// Internal: Lowers a field's arity.
    fn lower_field_arity(&self, field_arity: dml::FieldArity) -> ast::FieldArity {
        match field_arity {
            dml::FieldArity::Required => ast::FieldArity::Required,
            dml::FieldArity::Optional => ast::FieldArity::Optional,
            dml::FieldArity::List => ast::FieldArity::List,
        }
    }

    pub fn lower_default_value(dv: dml::DefaultValue) -> ast::Expression {
        match dv {
            dml::DefaultValue::Single(v) => Self::lower_prisma_value(&v),
            dml::DefaultValue::Expression(e) => {
                let exprs = e.args.iter().map(Self::lower_prisma_value).collect();
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
                vec.iter().map(|pv| Self::lower_prisma_value(pv)).collect(),
                ast::Span::empty(),
            ),
        }
    }

    /// Internal: Lowers a field's arity.
    fn lower_type(&self, field_type: &dml::FieldType) -> ast::Identifier {
        match field_type {
            dml::FieldType::Base(tpe, custom_type_name) => {
                ast::Identifier::new(&custom_type_name.as_ref().unwrap_or(&tpe.to_string()))
            }
            dml::FieldType::Enum(tpe) => ast::Identifier::new(&tpe.to_string()),
            dml::FieldType::Unsupported(tpe) => ast::Identifier::new(&tpe.to_string()),
            dml::FieldType::Relation(rel) => ast::Identifier::new(&rel.to),
            _ => unimplemented!("Connector specific types are not supported atm."),
        }
    }
}
