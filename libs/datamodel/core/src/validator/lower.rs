use super::{common::*, DirectiveBox};
use crate::configuration;
use crate::error::ErrorCollection;
use crate::{ast, dml};

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

    /// Creates a new instance, with all builtin directives and
    /// the directives defined by the given sources registered.
    ///
    /// The directives defined by the given sources will be namespaced.
    #[allow(unused)]
    fn with_sources(sources: &[Box<dyn configuration::Source + Send + Sync>]) -> LowerDmlToAst {
        LowerDmlToAst {
            directives: DirectiveBox::with_sources(sources),
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

    fn lower_model(&self, model: &dml::Model, datamodel: &dml::Datamodel) -> Result<ast::Model, ErrorCollection> {
        let mut errors = ErrorCollection::new();
        let mut fields: Vec<ast::Field> = Vec::new();

        for field in model.fields() {
            if !field.is_generated {
                match self.lower_field(field, model, datamodel) {
                    Ok(ast_field) => fields.push(ast_field),
                    Err(mut err) => errors.append(&mut err),
                };
            }
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
        })
    }

    fn lower_enum(&self, enm: &dml::Enum, datamodel: &dml::Datamodel) -> Result<ast::Enum, ErrorCollection> {
        Ok(ast::Enum {
            name: ast::Identifier::new(&enm.name),
            values: enm
                .values
                .iter()
                .map(|v| ast::EnumValue {
                    name: v.clone(),
                    span: ast::Span::empty(),
                })
                .collect(),
            directives: self.directives.enm.serialize(enm, datamodel)?,
            documentation: enm.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        })
    }

    fn lower_field(
        &self,
        field: &dml::Field,
        model: &dml::Model,
        datamodel: &dml::Datamodel,
    ) -> Result<ast::Field, ErrorCollection> {
        Ok(ast::Field {
            name: ast::Identifier::new(&field.name),
            arity: self.lower_field_arity(field.arity),
            default_value: field.default_value.clone().map(|dv| Self::lower_default_value(dv)),
            directives: self.directives.field.serialize(field, datamodel)?,
            field_type: self.lower_type(&field.field_type, field, model, &datamodel),
            documentation: field.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
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
            dml::DefaultValue::Single(v) => Self::lower_scalar_value(&v),
            dml::DefaultValue::Expression(e) => {
                let exprs = e.args.iter().map(Self::lower_scalar_value).collect();
                ast::Expression::Function(e.name, exprs, ast::Span::empty())
            }
        }
    }

    pub fn lower_scalar_value(sv: &dml::ScalarValue) -> ast::Expression {
        match sv {
            dml::ScalarValue::Boolean(true) => ast::Expression::BooleanValue(String::from("true"), ast::Span::empty()),
            dml::ScalarValue::Boolean(false) => {
                ast::Expression::BooleanValue(String::from("false"), ast::Span::empty())
            }
            dml::ScalarValue::String(value) => ast::Expression::StringValue(value.clone(), ast::Span::empty()),
            dml::ScalarValue::ConstantLiteral(value) => {
                ast::Expression::ConstantValue(value.clone(), ast::Span::empty())
            }
            dml::ScalarValue::DateTime(value) => ast::Expression::ConstantValue(value.to_rfc3339(), ast::Span::empty()),
            dml::ScalarValue::Decimal(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
            dml::ScalarValue::Float(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
            dml::ScalarValue::Int(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
        }
    }

    /// Internal: Lowers a field's arity.
    fn lower_type(
        &self,
        field_type: &dml::FieldType,
        field: &dml::Field,
        model: &dml::Model,
        datamodel: &dml::Datamodel,
    ) -> ast::Identifier {
        match field_type {
            dml::FieldType::Base(tpe) => ast::Identifier::new(&tpe.to_string()),
            dml::FieldType::Enum(tpe) => ast::Identifier::new(&tpe.to_string()),
            dml::FieldType::Relation(rel) => {
                let related_model = datamodel.find_model(&rel.to).expect(STATE_ERROR);

                if related_model.is_generated && related_model.is_pure_relation_model() {
                    // This is a special simplification case: We need to point to the original related field for rendering.
                    // This hides auto-generated relation tables.
                    let related_field = related_model
                        .related_field(&model.name, &rel.name, &field.name)
                        .expect(STATE_ERROR);
                    let other_field = related_model
                        .fields()
                        .find(|f| f.name != related_field.name)
                        .expect(STATE_ERROR);

                    if let dml::FieldType::Relation(rel) = &other_field.field_type {
                        ast::Identifier::new(&rel.to)
                    } else {
                        panic!(STATE_ERROR);
                    }
                } else if related_model.is_generated {
                    panic!("Error during rendering model: We found a relation to a generated model, but we do not know how to handle it. This is an internal error.")
                } else {
                    ast::Identifier::new(&rel.to)
                }
            }
            _ => unimplemented!("Connector specific types are not supported atm."),
        }
    }
}
