use crate::common::preview_features::PreviewFeature;
use crate::common::RelationNames;
use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self, Attribute, Span},
    dml, Datasource, Field, Ignorable, WithDatabaseName,
};
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

    /// Internal: Lowers a field's type.
    pub(crate) fn lower_type(&self, field_type: &dml::FieldType) -> ast::FieldType {
        match field_type {
            dml::FieldType::Scalar(tpe, custom_type_name, _) => ast::FieldType::Supported(ast::Identifier::new(
                custom_type_name.as_ref().unwrap_or(&tpe.to_string()),
            )),
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
    pub(crate) fn lower_field_attributes(&self, field: &dml::Field, datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        // @id
        if let dml::Field::ScalarField(sf) = field {
            if sf.is_id {
                attributes.push(ast::Attribute::new("id", Vec::new()));
            }
        }

        // @unique
        if let dml::Field::ScalarField(sf) = field {
            if sf.is_unique {
                attributes.push(ast::Attribute::new("unique", vec![]));
            }
        }

        // @default
        if let Some(default_value) = field.default_value() {
            attributes.push(ast::Attribute::new(
                "default",
                vec![ast::Argument::new(
                    "",
                    LowerDmlToAst::<'a>::lower_default_value(default_value.clone()),
                )],
            ));
        }

        // @updatedAt
        if field.is_updated_at() {
            attributes.push(ast::Attribute::new("updatedAt", Vec::new()));
        }

        // @map
        if let Some(db_name) = field.database_name() {
            attributes.push(ast::Attribute::new(
                "map",
                vec![ast::Argument::new_unnamed(ast::Expression::StringValue(
                    String::from(db_name),
                    Span::empty(),
                ))],
            ));
        }

        // @relation
        if let dml::Field::RelationField(rf) = field {
            let mut args = Vec::new();
            let relation_info = &rf.relation_info;
            let parent_model = datamodel.find_model_by_relation_field_ref(rf).unwrap();

            let related_model = datamodel
                .find_model(&relation_info.to)
                .unwrap_or_else(|| panic!("Related model not found: {}.", relation_info.to));

            let mut all_related_ids = related_model.id_field_names();
            let has_default_name = relation_info.name
                == RelationNames::name_for_unambiguous_relation(&relation_info.to, &parent_model.name);

            if !relation_info.name.is_empty() && (!has_default_name || parent_model.name == related_model.name) {
                args.push(ast::Argument::new_string("", &relation_info.name));
            }

            let mut relation_fields = relation_info.references.clone();

            relation_fields.sort();
            all_related_ids.sort();

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

            if self.preview_features.contains(PreviewFeature::ReferentialActions) {
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
        match dv {
            dml::DefaultValue::Single(v) => LowerDmlToAst::<'a>::lower_prisma_value(&v),
            dml::DefaultValue::Expression(e) => {
                let exprs = e.args.iter().map(LowerDmlToAst::<'a>::lower_prisma_value).collect();
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
