use super::super::directives::AllDirectives;
use crate::configuration::preview_features::PreviewFeatures;
use crate::error::ErrorCollection;
use crate::FieldType::NativeType;
use crate::{ast, dml, Datasource};

pub struct LowerDmlToAst {
    directives: AllDirectives,
    datasource: Option<Datasource>,
}

impl LowerDmlToAst {
    /// Creates a new instance, with all builtin directives registered.
    pub fn new(datasource: Option<Datasource>) -> Self {
        Self {
            directives: AllDirectives::new(),
            datasource,
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

        Ok(ast::SchemaAst { tops })
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
                .values()
                .map(|v| ast::EnumValue {
                    name: ast::Identifier::new(&v.name),
                    directives: self.directives.enm_value.serialize(v, datamodel).unwrap(),
                    documentation: v.documentation.clone().map(|text| ast::Comment { text }),
                    span: ast::Span::empty(),
                    commented_out: v.commented_out,
                })
                .collect(),
            directives: self.directives.enm.serialize(enm, datamodel)?,
            documentation: enm.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        })
    }

    pub fn lower_field(&self, field: &dml::Field, datamodel: &dml::Datamodel) -> Result<ast::Field, ErrorCollection> {
        println!("{:?}", field);
        /*let (is_native_type_field: bool, native_directive: Option<ast::Directive>) = if let dml::Field::ScalarField(sf) = field {
            let rr = self.datasource.unwrap();
            if let NativeType(prismaT, nativeT) = sf.clone().field_type {
                if rr.has_preview_feature("nativeTypes") {
                    let name = format!("{}.{}", rr.name, sf.name);
                }
                println!("datamodel: {:?}", datamodel);
                (true, ast::Directive::new(name, vec![]))
            }
        } else {
            (false, None)
        };*/
        let is_native_type_field = true;
        let directives: Vec<ast::Directive> = if is_native_type_field {
            vec![ast::Directive::new("test", vec![])]
        } else {
            self.directives.field.serialize(field, datamodel)?
        };

        Ok(ast::Field {
            name: ast::Identifier::new(&field.name()),
            arity: self.lower_field_arity(field.arity()),
            directives,
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
