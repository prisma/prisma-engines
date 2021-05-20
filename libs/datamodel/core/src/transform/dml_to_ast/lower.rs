use super::super::attributes::AllAttributes;
use crate::{
    ast::{self, Attribute, Span},
    dml, Datasource,
};

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

    pub fn lower(&self, datamodel: &dml::Datamodel) -> ast::SchemaAst {
        let mut tops: Vec<ast::Top> = Vec::new();

        for model in datamodel.models() {
            if !model.is_generated {
                tops.push(ast::Top::Model(self.lower_model(model, datamodel)))
            }
        }

        for enm in datamodel.enums() {
            tops.push(ast::Top::Enum(self.lower_enum(enm, datamodel)))
        }

        ast::SchemaAst { tops }
    }

    pub fn lower_model(&self, model: &dml::Model, datamodel: &dml::Datamodel) -> ast::Model {
        let mut fields: Vec<ast::Field> = Vec::new();

        for field in model.fields() {
            fields.push(self.lower_field(field, datamodel))
        }

        ast::Model {
            name: ast::Identifier::new(&model.name),
            fields,
            attributes: self.attributes.model.serialize(model, datamodel),
            documentation: model.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
            commented_out: model.is_commented_out,
        }
    }

    fn lower_enum(&self, enm: &dml::Enum, datamodel: &dml::Datamodel) -> ast::Enum {
        ast::Enum {
            name: ast::Identifier::new(&enm.name),
            values: enm
                .values()
                .map(|v| ast::EnumValue {
                    name: ast::Identifier::new(&v.name),
                    attributes: self.attributes.enm_value.serialize(v, datamodel),
                    documentation: v.documentation.clone().map(|text| ast::Comment { text }),
                    span: ast::Span::empty(),
                    commented_out: v.commented_out,
                })
                .collect(),
            attributes: self.attributes.enm.serialize(enm, datamodel),
            documentation: enm.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        }
    }

    pub fn lower_field(&self, field: &dml::Field, datamodel: &dml::Datamodel) -> ast::Field {
        let mut attributes = self.attributes.field.serialize(field, datamodel);

        if let (Some((scalar_type, native_type)), Some(datasource)) = (
            field.as_scalar_field().and_then(|sf| sf.field_type.as_native_type()),
            self.datasource,
        ) {
            self.lower_native_type_attribute(scalar_type, native_type, &mut attributes, datasource);
        }

        ast::Field {
            name: ast::Identifier::new(&field.name()),
            arity: self.lower_field_arity(field.arity()),
            attributes,
            field_type: self.lower_type(&field.field_type()),
            documentation: field.documentation().map(|text| ast::Comment { text: text.to_owned() }),
            span: ast::Span::empty(),
            is_commented_out: field.is_commented_out(),
        }
    }

    /// Internal: Lowers a field's arity.
    fn lower_field_arity(&self, field_arity: &dml::FieldArity) -> ast::FieldArity {
        match field_arity {
            dml::FieldArity::Required => ast::FieldArity::Required,
            dml::FieldArity::Optional => ast::FieldArity::Optional,
            dml::FieldArity::List => ast::FieldArity::List,
        }
    }

    /// Internal: Lowers a field's type.
    fn lower_type(&self, field_type: &dml::FieldType) -> ast::FieldType {
        match field_type {
            dml::FieldType::Base(tpe, custom_type_name) => ast::FieldType::Supported(ast::Identifier::new(
                &custom_type_name.as_ref().unwrap_or(&tpe.to_string()),
            )),
            dml::FieldType::Enum(tpe) => ast::FieldType::Supported(ast::Identifier::new(&tpe)),
            dml::FieldType::Unsupported(tpe) => ast::FieldType::Unsupported(tpe.clone(), Span::empty()),
            dml::FieldType::Relation(rel) => ast::FieldType::Supported(ast::Identifier::new(&rel.to)),
            dml::FieldType::NativeType(prisma_tpe, _native_tpe) => {
                ast::FieldType::Supported(ast::Identifier::new(&prisma_tpe.to_string()))
            }
        }
    }

    fn lower_native_type_attribute(
        &self,
        scalar_type: &dml::ScalarType,
        native_type: &dml::NativeTypeInstance,
        attributes: &mut Vec<Attribute>,
        datasource: &Datasource,
    ) {
        if datasource
            .active_connector
            .native_type_is_default_for_scalar_type(native_type.serialized_native_type.clone(), scalar_type)
        {
            return;
        }

        let new_attribute_name = format!("{}.{}", datasource.name, native_type.name);
        let arguments = native_type
            .args
            .iter()
            .map(|arg| ast::Argument::new_unnamed(ast::Expression::NumericValue(arg.to_owned(), Span::empty())))
            .collect();

        attributes.push(ast::Attribute::new(new_attribute_name.as_str(), arguments));
    }
}
