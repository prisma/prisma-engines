use crate::common::constraint_names::ConstraintNames;
use crate::common::RelationNames;
use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self, Attribute, Span},
    dml, Datasource, Field, Ignorable,
};
use ::dml::traits::WithName;
use datamodel_connector::{Connector, EmptyDatamodelConnector};
use prisma_value::PrismaValue;

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers a field's arity.
    pub(crate) fn lower_field_arity(&self, field_arity: &dml::FieldArity) -> ast::FieldArity {
        match field_arity {
            dml::FieldArity::Required => ast::FieldArity::Required,
            dml::FieldArity::Optional => ast::FieldArity::Optional,
            dml::FieldArity::List => ast::FieldArity::List,
        }
    }

    pub(crate) fn lower_composite_field_type(&self, field_type: &dml::CompositeTypeFieldType) -> ast::FieldType {
        match field_type {
            ::dml::composite_type::CompositeTypeFieldType::CompositeType(name) => {
                ast::FieldType::Supported(ast::Identifier::new(name))
            }
            ::dml::composite_type::CompositeTypeFieldType::Unsupported(name) => {
                ast::FieldType::Unsupported(name.clone(), Span::empty())
            }
            ::dml::composite_type::CompositeTypeFieldType::Scalar(tpe, custom_type_name, _) => {
                ast::FieldType::Supported(ast::Identifier::new(
                    custom_type_name.as_ref().unwrap_or(&tpe.to_string()),
                ))
            }
        }
    }

    /// Internal: Lowers a field's type.
    pub(crate) fn lower_type(&self, field_type: &dml::FieldType) -> ast::FieldType {
        match field_type {
            dml::FieldType::Scalar(tpe, custom_type_name, _) => ast::FieldType::Supported(ast::Identifier::new(
                custom_type_name.as_ref().unwrap_or(&tpe.to_string()),
            )),
            dml::FieldType::CompositeType(name) => ast::FieldType::Supported(ast::Identifier::new(name)),
            dml::FieldType::Enum(tpe) => ast::FieldType::Supported(ast::Identifier::new(tpe)),
            dml::FieldType::Unsupported(tpe) => ast::FieldType::Unsupported(tpe.clone(), Span::empty()),
            dml::FieldType::Relation(rel) => ast::FieldType::Supported(ast::Identifier::new(&rel.to)),
        }
    }

    pub(crate) fn lower_native_type_attribute(
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

    /// Internal: Lowers a field's attributes.
    pub(crate) fn lower_field_attributes(
        &self,
        model: &dml::Model,
        field: &dml::Field,
        datamodel: &dml::Datamodel,
    ) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        // @id
        if let dml::Field::ScalarField(sf) = field {
            if model.field_is_primary_and_defined_on_field(&sf.name) {
                let mut args = Vec::new();
                let pk = model.primary_key.as_ref().unwrap();
                if pk.db_name.is_some() {
                    if let Some(src) = self.datasource {
                        if !ConstraintNames::primary_key_name_matches(pk, model, &*src.active_connector) {
                            args.push(ast::Argument::new(
                                "map",
                                ast::Expression::StringValue(String::from(pk.db_name.as_ref().unwrap()), Span::empty()),
                            ));
                        }
                    };
                }

                attributes.push(ast::Attribute::new("id", args));
            }
        }

        // @unique
        if let dml::Field::ScalarField(sf) = field {
            if model.field_is_unique_and_defined_on_field(&sf.name) {
                let mut arguments = Vec::new();
                if let Some(idx) = model
                    .indices
                    .iter()
                    .find(|id| id.is_unique() && id.defined_on_field && id.fields == [field.name()])
                {
                    self.push_index_map_argument(model, idx, &mut arguments)
                }

                attributes.push(ast::Attribute::new("unique", arguments));
            }
        }

        // @default
        if let Some(default_value) = field.default_value() {
            let mut args = vec![ast::Argument::new(
                "",
                LowerDmlToAst::<'a>::lower_default_value(default_value.clone()),
            )];

            let connector = self
                .datasource
                .map(|source| source.active_connector.as_ref())
                .unwrap_or(&EmptyDatamodelConnector as &dyn Connector);

            let prisma_default = ConstraintNames::default_name(model.name(), field.name(), connector);

            if let Some(name) = default_value.db_name() {
                if name != prisma_default {
                    args.push(ast::Argument::new("map", Self::lower_string(name)))
                }
            }

            attributes.push(ast::Attribute::new("default", args));
        }

        // @updatedAt
        if field.is_updated_at() {
            attributes.push(ast::Attribute::new("updatedAt", Vec::new()));
        }

        // @map
        <LowerDmlToAst<'a>>::push_map_attribute(field, &mut attributes);

        // @relation
        if let dml::Field::RelationField(rf) = field {
            let mut args = Vec::new();
            let relation_info = &rf.relation_info;
            let parent_model = datamodel.find_model_by_relation_field_ref(rf).unwrap();

            let related_model = datamodel
                .find_model(&relation_info.to)
                .unwrap_or_else(|| panic!("Related model not found: {}.", relation_info.to));

            let has_default_name = relation_info.name
                == RelationNames::name_for_unambiguous_relation(&relation_info.to, &parent_model.name);

            if !relation_info.name.is_empty() && (!has_default_name || parent_model.name == related_model.name) {
                args.push(ast::Argument::new_string("", relation_info.name.to_string()));
            }

            let mut relation_fields = relation_info.references.clone();

            relation_fields.sort();

            if !relation_info.fields.is_empty() {
                args.push(ast::Argument::new_array(
                    "fields",
                    LowerDmlToAst::field_array(&relation_info.fields),
                ));
            }

            // if we are on the physical field
            if !relation_info.references.is_empty() {
                let is_many_to_many = match &field {
                    Field::RelationField(relation_field) => {
                        let (_, related_field) = datamodel.find_related_field(relation_field).unwrap();
                        relation_field.arity.is_list() && related_field.arity.is_list()
                    }
                    _ => false,
                };

                if !is_many_to_many {
                    args.push(ast::Argument::new_array(
                        "references",
                        LowerDmlToAst::field_array(&relation_info.references),
                    ));
                }
            }

            if let Some(ref_action) = relation_info.on_delete {
                if rf.default_on_delete_action() != ref_action {
                    let expression = ast::Expression::ConstantValue(ref_action.to_string(), ast::Span::empty());
                    args.push(ast::Argument::new("onDelete", expression));
                }
            }

            if let Some(ref_action) = relation_info.on_update {
                if rf.default_on_update_action() != ref_action {
                    let expression = ast::Expression::ConstantValue(ref_action.to_string(), ast::Span::empty());
                    args.push(ast::Argument::new("onUpdate", expression));
                }
            }

            if let Some(fk_name) = &relation_info.fk_name {
                if let Some(src) = self.datasource {
                    if !ConstraintNames::foreign_key_name_matches(relation_info, model, &*src.active_connector) {
                        args.push(ast::Argument::new(
                            "map",
                            ast::Expression::StringValue(String::from(fk_name), Span::empty()),
                        ));
                    }
                };
            }

            if !args.is_empty() {
                attributes.push(ast::Attribute::new("relation", args));
            }
        }

        // @ignore
        if field.is_ignored() {
            attributes.push(ast::Attribute::new("ignore", vec![]));
        }

        attributes
    }

    pub fn lower_default_value(dv: dml::DefaultValue) -> ast::Expression {
        match dv.kind() {
            dml::DefaultKind::Single(v) => LowerDmlToAst::<'a>::lower_prisma_value(v),
            dml::DefaultKind::Expression(e) => {
                let exprs = e.args().iter().map(LowerDmlToAst::<'a>::lower_prisma_value).collect();
                ast::Expression::Function(e.name().to_string(), exprs, ast::Span::empty())
            }
        }
    }

    pub fn lower_string(s: impl ToString) -> ast::Expression {
        ast::Expression::StringValue(s.to_string(), ast::Span::empty())
    }

    pub fn lower_prisma_value(pv: &PrismaValue) -> ast::Expression {
        match pv {
            PrismaValue::Boolean(true) => ast::Expression::BooleanValue(String::from("true"), ast::Span::empty()),
            PrismaValue::Boolean(false) => ast::Expression::BooleanValue(String::from("false"), ast::Span::empty()),
            PrismaValue::String(value) => Self::lower_string(value),
            PrismaValue::Enum(value) => ast::Expression::ConstantValue(value.clone(), ast::Span::empty()),
            PrismaValue::DateTime(value) => Self::lower_string(value),
            PrismaValue::Float(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
            PrismaValue::Int(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
            PrismaValue::BigInt(value) => ast::Expression::NumericValue(value.to_string(), ast::Span::empty()),
            PrismaValue::Null => ast::Expression::ConstantValue("null".to_string(), ast::Span::empty()),
            PrismaValue::Uuid(val) => Self::lower_string(val),
            PrismaValue::Json(val) => Self::lower_string(val),
            PrismaValue::List(vec) => ast::Expression::Array(
                vec.iter()
                    .map(|pv| LowerDmlToAst::<'a>::lower_prisma_value(pv))
                    .collect(),
                ast::Span::empty(),
            ),
            PrismaValue::Xml(val) => ast::Expression::StringValue(val.to_string(), ast::Span::empty()),
            PrismaValue::Bytes(b) => ast::Expression::StringValue(prisma_value::encode_bytes(b), ast::Span::empty()),
        }
    }
}
