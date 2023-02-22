use crate::serialization_ast::datamodel_ast::{
    Datamodel, Enum, EnumValue, Field, Function, Model, PrimaryKey, UniqueIndex,
};
use bigdecimal::ToPrimitive;
use prisma_models::dml::{self, PrismaValue};
use psl::{
    parser_database::{walkers, ScalarFieldType},
    schema_ast::ast::WithDocumentation,
};

pub fn schema_to_dmmf(schema: &psl::ValidatedSchema) -> Datamodel {
    let mut datamodel = Datamodel {
        models: Vec::with_capacity(schema.db.models_count()),
        enums: Vec::with_capacity(schema.db.enums_count()),
        types: Vec::new(),
    };

    for enum_model in schema.db.walk_enums() {
        datamodel.enums.push(enum_to_dmmf(enum_model));
    }

    for model in schema
        .db
        .walk_models()
        .filter(|model| !model.is_ignored())
        .chain(schema.db.walk_views())
    {
        datamodel.models.push(model_to_dmmf(model));
    }

    for ct in schema.db.walk_composite_types() {
        datamodel.types.push(composite_type_to_dmmf(ct))
    }

    datamodel
}

fn enum_to_dmmf(en: walkers::EnumWalker<'_>) -> Enum {
    let mut enm = Enum {
        name: en.name().to_owned(),
        values: vec![],
        db_name: en.mapped_name().map(ToOwned::to_owned),
        documentation: en.ast_enum().documentation().map(ToOwned::to_owned),
    };

    for enum_value in en.values() {
        enm.values.push(enum_value_to_dmmf(enum_value));
    }

    enm
}

fn enum_value_to_dmmf(en: walkers::EnumValueWalker<'_>) -> EnumValue {
    EnumValue {
        name: en.name().to_owned(),
        db_name: en.mapped_name().map(ToOwned::to_owned),
    }
}

fn composite_type_to_dmmf(ct: walkers::CompositeTypeWalker<'_>) -> Model {
    Model {
        name: ct.name().to_owned(),
        db_name: None,
        fields: ct
            .fields()
            .filter(|field| !matches!(field.r#type(), ScalarFieldType::Unsupported(_)))
            .map(composite_type_field_to_dmmf)
            .collect(),
        is_generated: None,
        documentation: None,
        primary_key: None,
        unique_fields: Vec::new(),
        unique_indexes: Vec::new(),
    }
}

fn composite_type_field_to_dmmf(field: walkers::CompositeTypeFieldWalker<'_>) -> Field {
    Field {
        name: field.name().to_owned(),
        kind: match field.r#type() {
            ScalarFieldType::CompositeType(_) => "object",
            ScalarFieldType::Enum(_) => "enum",
            ScalarFieldType::BuiltInScalar(_) => "scalar",
            ScalarFieldType::Unsupported(_) => unreachable!(),
        },
        db_name: field.mapped_name().map(ToOwned::to_owned),
        is_required: field.arity() == dml::FieldArity::Required || field.arity() == dml::FieldArity::List,
        is_list: field.arity() == dml::FieldArity::List,
        is_id: false,
        is_read_only: false,
        has_default_value: field.default_value().is_some(),
        default: field
            .default_value()
            .map(|dv| default_value_to_serde(&dml::dml_default_kind(dv, field.scalar_type()))),
        is_unique: false,
        relation_name: None,
        relation_from_fields: None,
        relation_to_fields: None,
        relation_on_delete: None,
        field_type: match field.r#type() {
            ScalarFieldType::CompositeType(ct) => field.walk(ct).name().to_owned(),
            ScalarFieldType::Enum(enm) => field.walk(enm).name().to_owned(),
            ScalarFieldType::BuiltInScalar(st) => st.as_str().to_owned(),
            ScalarFieldType::Unsupported(_) => unreachable!(),
        },
        is_generated: None,
        is_updated_at: None,
        documentation: None,
    }
}

fn model_to_dmmf(model: walkers::ModelWalker<'_>) -> Model {
    let primary_key = if let Some(pk) = model.primary_key() {
        (!pk.is_defined_on_field()).then(|| PrimaryKey {
            name: pk.name().map(ToOwned::to_owned),
            fields: pk.fields().map(|f| f.name().to_owned()).collect(),
        })
    } else {
        None
    };

    Model {
        name: model.name().to_owned(),
        db_name: model.mapped_name().map(ToOwned::to_owned),
        fields: model
            .fields()
            .filter(|field| !should_skip_model_field(field))
            .map(field_to_dmmf)
            .collect(),
        is_generated: Some(false),
        documentation: model.ast_model().documentation().map(ToOwned::to_owned),
        primary_key,
        unique_fields: model
            .indexes()
            .filter_map(|i| {
                (i.is_unique() && !i.is_defined_on_field()).then(|| i.fields().map(|f| f.name().to_owned()).collect())
            })
            .collect(),
        unique_indexes: model
            .indexes()
            .filter_map(|i| {
                (i.is_unique() && !i.is_defined_on_field()).then(|| UniqueIndex {
                    name: i.name().map(ToOwned::to_owned),
                    fields: i.fields().map(|f| f.name().to_owned()).collect(),
                })
            })
            .collect(),
    }
}

fn should_skip_model_field(field: &walkers::FieldWalker<'_>) -> bool {
    match field.refine() {
        walkers::RefinedFieldWalker::Scalar(f) => f.is_ignored() || f.is_unsupported(),
        walkers::RefinedFieldWalker::Relation(f) => f.is_ignored(),
    }
}

fn field_to_dmmf(field: walkers::FieldWalker<'_>) -> Field {
    match field.refine() {
        walkers::RefinedFieldWalker::Scalar(sf) => scalar_field_to_dmmf(sf),
        walkers::RefinedFieldWalker::Relation(rf) => relation_field_to_dmmf(rf),
    }
}

fn scalar_field_to_dmmf(field: walkers::ScalarFieldWalker<'_>) -> Field {
    let ast_field = field.ast_field();
    let field_walker = walkers::FieldWalker::from(field);
    let is_id = field.is_single_pk();
    Field {
        name: field.name().to_owned(),
        db_name: field.mapped_name().map(ToOwned::to_owned),
        kind: match field.scalar_field_type() {
            ScalarFieldType::CompositeType(_) => "object",
            ScalarFieldType::Enum(_) => "enum",
            ScalarFieldType::BuiltInScalar(_) => "scalar",
            ScalarFieldType::Unsupported(_) => unreachable!(),
        },
        is_list: ast_field.arity.is_list(),
        is_required: matches!(ast_field.arity, dml::FieldArity::Required | dml::FieldArity::List),
        is_unique: !is_id && field.is_unique(),
        is_id,
        is_read_only: field.model().relation_fields().any(|rf| {
            rf.referencing_fields()
                .into_iter()
                .flatten()
                .any(|f| f.field_id() == field.field_id())
        }),
        has_default_value: field.default_value().is_some(),
        field_type: match field.scalar_field_type() {
            ScalarFieldType::CompositeType(ct) => field_walker.walk(ct).name().to_owned(),
            ScalarFieldType::Enum(enm) => field_walker.walk(enm).name().to_owned(),
            ScalarFieldType::BuiltInScalar(st) => st.as_str().to_owned(),
            ScalarFieldType::Unsupported(_) => unreachable!(),
        },
        default: field
            .default_value()
            .map(|dv| default_value_to_serde(&dml::dml_default_kind(dv.value(), field.scalar_type()))),
        relation_name: None,
        relation_from_fields: None,
        relation_to_fields: None,
        relation_on_delete: None,
        is_generated: Some(false),
        is_updated_at: Some(field.is_updated_at()),
        documentation: ast_field.documentation().map(ToOwned::to_owned),
    }
}

fn relation_field_to_dmmf(field: walkers::RelationFieldWalker<'_>) -> Field {
    let ast_field = field.ast_field();
    Field {
        name: field.name().to_owned(),
        db_name: None,
        kind: "object",
        is_list: ast_field.arity.is_list(),
        is_required: matches!(ast_field.arity, dml::FieldArity::Required | dml::FieldArity::List),
        is_unique: false,
        is_id: false,
        is_read_only: false,
        has_default_value: false,
        field_type: field.related_model().name().to_owned(),
        default: None,
        relation_name: Some(field.relation_name().to_string()),
        relation_from_fields: Some(
            field
                .referencing_fields()
                .map(|fields| fields.map(|f| f.name().to_owned()).collect())
                .unwrap_or_default(),
        ),
        relation_to_fields: Some(
            field
                .referenced_fields()
                .map(|fields| fields.map(|f| f.name().to_owned()).collect())
                .unwrap_or_default(),
        ),
        relation_on_delete: field.explicit_on_delete().map(|od| od.to_string()),
        is_generated: Some(false),
        is_updated_at: Some(false),
        documentation: ast_field.documentation().map(ToOwned::to_owned),
    }
}

fn default_value_to_serde(dv: &dml::DefaultKind) -> serde_json::Value {
    match dv {
        dml::DefaultKind::Single(value) => prisma_value_to_serde(&value.clone()),
        dml::DefaultKind::Expression(vg) => {
            let args: Vec<_> = vg.args().iter().map(|(_, v)| v.clone()).collect();
            function_to_serde(vg.name(), &args)
        }
    }
}

fn prisma_value_to_serde(value: &PrismaValue) -> serde_json::Value {
    match value {
        PrismaValue::Boolean(val) => serde_json::Value::Bool(*val),
        PrismaValue::String(val) => serde_json::Value::String(val.clone()),
        PrismaValue::Enum(val) => serde_json::Value::String(val.clone()),
        PrismaValue::Float(val) => {
            serde_json::Value::Number(serde_json::Number::from_f64(val.to_f64().unwrap()).unwrap())
        }
        PrismaValue::Int(val) => serde_json::Value::Number(serde_json::Number::from_f64(*val as f64).unwrap()),
        PrismaValue::BigInt(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::DateTime(val) => serde_json::Value::String(val.to_rfc3339()),
        PrismaValue::Null => serde_json::Value::Null,
        PrismaValue::Uuid(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::Json(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::Xml(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::List(value_vec) => serde_json::Value::Array(value_vec.iter().map(prisma_value_to_serde).collect()),
        PrismaValue::Bytes(b) => serde_json::Value::String(dml::prisma_value::encode_bytes(b)),
        PrismaValue::Object(pairs) => {
            let mut map = serde_json::Map::with_capacity(pairs.len());
            pairs.iter().for_each(|(key, value)| {
                map.insert(key.clone(), prisma_value_to_serde(value));
            });

            serde_json::Value::Object(map)
        }
    }
}

fn function_to_serde(name: &str, args: &[PrismaValue]) -> serde_json::Value {
    let func = Function {
        name: String::from(name),
        args: args.iter().map(prisma_value_to_serde).collect(),
    };

    serde_json::to_value(&func).expect("Failed to render function JSON")
}

#[cfg(test)]
mod tests {
    use super::schema_to_dmmf;
    use pretty_assertions::assert_eq;
    use std::fs;

    fn render_to_dmmf(schema: &str) -> String {
        let schema = psl::parse_schema(schema).unwrap();
        let dmmf = schema_to_dmmf(&schema);
        serde_json::to_string_pretty(&dmmf).expect("Failed to render JSON")
    }

    #[test]
    fn test_dmmf_rendering() {
        let test_cases = vec![
            "general",
            "functions",
            "source",
            "source_with_comments",
            "source_with_generator",
            "without_relation_name",
            "ignore",
            "views",
        ];

        for test_case in test_cases {
            println!("TESTING: {test_case}");

            let datamodel_string = load_from_file(format!("{test_case}.prisma").as_str());
            let dmmf_string = render_to_dmmf(&datamodel_string);
            let expected_json = load_from_file(format!("{test_case}.json").as_str());
            println!("{dmmf_string}");
            assert_eq_json(&dmmf_string, &expected_json, test_case);
        }
    }

    #[track_caller]
    fn assert_eq_json(a: &str, b: &str, msg: &str) {
        let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
        let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

        assert_eq!(json_a, json_b, "{}", msg);
    }

    fn load_from_file(file: &str) -> String {
        let samples_folder_path = concat!(env!("CARGO_MANIFEST_DIR"), "/test_files");
        fs::read_to_string(format!("{samples_folder_path}/{file}")).unwrap()
    }
}
