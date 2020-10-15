use super::super::attributes::AllAttributes;
use crate::ast::Span;
use crate::configuration::preview_features::PreviewFeatures;
use crate::diagnostics::Diagnostics;
use crate::dml::FieldType::NativeType;
use crate::{ast, dml, Datasource};

pub struct LowerDmlToAst<'a> {
    attributes: AllAttributes,
    datasource: Option<&'a Datasource>,
}

impl<'a> LowerDmlToAst<'a> {
    /// Creates a new instance, with all builtin attributes registered.
    pub fn new(datasource: Option<&'a Datasource>) -> Self {
        Self {
            attributes: AllAttributes::new(),
            datasource,
        }
    }

    pub fn lower(&self, datamodel: &dml::Datamodel) -> Result<ast::SchemaAst, Diagnostics> {
        let mut tops: Vec<ast::Top> = Vec::new();
        let mut errors = Diagnostics::new();

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

        Ok(ast::SchemaAst { tops })
    }

    pub fn lower_model(&self, model: &dml::Model, datamodel: &dml::Datamodel) -> Result<ast::Model, Diagnostics> {
        let mut errors = Diagnostics::new();
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
            attributes: self.attributes.model.serialize(model, datamodel)?,
            documentation: model.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
            commented_out: model.is_commented_out,
        })
    }

    fn lower_enum(&self, enm: &dml::Enum, datamodel: &dml::Datamodel) -> Result<ast::Enum, Diagnostics> {
        Ok(ast::Enum {
            name: ast::Identifier::new(&enm.name),
            values: enm
                .values()
                .map(|v| ast::EnumValue {
                    name: ast::Identifier::new(&v.name),
                    attributes: self.attributes.enm_value.serialize(v, datamodel).unwrap(),
                    documentation: v.documentation.clone().map(|text| ast::Comment { text }),
                    span: ast::Span::empty(),
                    commented_out: v.commented_out,
                })
                .collect(),
            attributes: self.attributes.enm.serialize(enm, datamodel)?,
            documentation: enm.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        })
    }

    pub fn lower_field(&self, field: &dml::Field, datamodel: &dml::Datamodel) -> Result<ast::Field, Diagnostics> {
        let mut attributes = self.attributes.field.serialize(field, datamodel)?;
        if let (dml::Field::ScalarField(sf), Some(datasource)) = (field, self.datasource) {
            if let NativeType(_prisma_tpe, native_tpe) = sf.clone().field_type {
                if datasource.has_preview_feature("nativeTypes") {
                    let new_attribute_name = format!("{}.{}", datasource.name, native_tpe.name);
                    // lower native type arguments
                    let mut arguments = vec![];
                    for arg in native_tpe.args {
                        arguments.push(ast::Argument::new_unnamed(ast::Expression::NumericValue(
                            arg.to_string(),
                            Span::empty(),
                        )));
                    }
                    attributes.push(ast::Attribute::new(new_attribute_name.as_str(), arguments));
                }
            }
        }

        Ok(ast::Field {
            name: ast::Identifier::new(&field.name()),
            arity: self.lower_field_arity(field.arity()),
            attributes,
            field_type: self.lower_type(&field.field_type()),
            documentation: field
                .documentation()
                .map(|text| ast::Comment { text: text.to_string() }),
            span: ast::Span::empty(),
            is_commented_out: field.is_commented_out(),
        })
    }

    /// Internal: Lowers a field's arity.
    fn lower_field_arity(&self, field_arity: &dml::FieldArity) -> ast::FieldArity {
        match field_arity {
            dml::FieldArity::Required => ast::FieldArity::Required,
            dml::FieldArity::Optional => ast::FieldArity::Optional,
            dml::FieldArity::List => ast::FieldArity::List,
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
            dml::FieldType::NativeType(prisma_tpe, _native_tpe) => ast::Identifier::new(&prisma_tpe.to_string()),
        }
    }
}
