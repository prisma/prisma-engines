use crate::serialization_ast::datamodel_ast::{
    Datamodel, Enum, EnumValue, Field, Function, Model, PrimaryKey, UniqueIndex,
};
use bigdecimal::ToPrimitive;
use prisma_models::dml::{self, FieldType, Ignorable, PrismaValue, ScalarType, WithDatabaseName};
use psl::{
    parser_database::{walkers, ScalarFieldType},
    schema_ast::ast::WithDocumentation,
};

pub fn schema_to_dmmf(schema: &psl::ValidatedSchema) -> Datamodel {
    let dml = dml::lift(schema);
    let mut datamodel = Datamodel {
        models: vec![],
        enums: vec![],
        types: Vec::with_capacity(dml.composite_types.len()),
    };

    for enum_model in schema.db.walk_enums() {
        datamodel.enums.push(enum_to_dmmf(enum_model));
    }

    for model in dml.models().filter(|model| !model.is_ignored) {
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
            ScalarFieldType::CompositeType(_) => "object".into(),
            ScalarFieldType::Enum(_) => "enum".into(),
            ScalarFieldType::BuiltInScalar(_) => "scalar".into(),
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
            ScalarFieldType::CompositeType(ct) => field.walk(*ct).name().to_owned(),
            ScalarFieldType::Enum(enm) => field.walk(*enm).name().to_owned(),
            ScalarFieldType::BuiltInScalar(st) => st.as_str().to_owned(),
            ScalarFieldType::Unsupported(_) => unreachable!(),
        },
        is_generated: None,
        is_updated_at: None,
        documentation: None,
    }
}

fn model_to_dmmf(model: &dml::Model) -> Model {
    let primary_key = if let Some(pk) = &model.primary_key {
        (!pk.defined_on_field).then(|| PrimaryKey {
            name: pk.name.clone(),
            //TODO(extended indices) add field options here
            fields: pk.fields.clone().into_iter().map(|f| f.name).collect(),
        })
    } else {
        None
    };

    Model {
        name: model.name.clone(),
        db_name: model.database_name.clone(),
        fields: model
            .fields()
            .filter(|field| !field.is_ignored() && !matches!(field.field_type(), FieldType::Unsupported(_)))
            .map(|f| field_to_dmmf(model, f))
            .collect(),
        is_generated: Some(model.is_generated),
        documentation: model.documentation.clone(),
        primary_key,
        unique_fields: model
            .indices
            .iter()
            .filter_map(|i| {
                (i.is_unique() && !i.defined_on_field).then(|| {
                    i.fields
                        .clone()
                        .into_iter()
                        .map(|f| f.path.into_iter().map(|(field, _)| field).collect::<Vec<_>>().join("."))
                        .collect()
                })
            })
            .collect(),
        unique_indexes: model
            .indices
            .iter()
            .filter_map(|i| {
                (i.is_unique() && !i.defined_on_field).then(|| UniqueIndex {
                    name: i.name.clone(),
                    //TODO(extended indices) add field options here
                    fields: i
                        .fields
                        .clone()
                        .into_iter()
                        .map(|f| f.path.into_iter().map(|(field, _)| field).collect::<Vec<_>>().join("."))
                        .collect(),
                })
            })
            .collect(),
    }
}

fn field_to_dmmf(model: &dml::Model, field: &dml::Field) -> Field {
    let a_relation_field_is_based_on_this_field: bool = model
        .relation_fields()
        .any(|f| f.relation_info.fields.iter().any(|f| f == field.name()));

    Field {
        name: field.name().to_string(),
        db_name: field.database_name().map(|f| f.to_string()),
        kind: get_field_kind(field),
        is_required: *field.arity() == dml::FieldArity::Required || *field.arity() == dml::FieldArity::List,
        is_list: *field.arity() == dml::FieldArity::List,
        is_id: model.field_is_primary(field.name()),
        is_read_only: a_relation_field_is_based_on_this_field,
        has_default_value: field.default_value().is_some(),
        default: field.default_value().map(|dv| default_value_to_serde(dv.kind())),
        is_unique: model.field_is_unique(field.name()),
        relation_name: get_relation_name(field),
        relation_from_fields: get_relation_from_fields(field),
        relation_to_fields: get_relation_to_fields(field),
        relation_on_delete: get_relation_delete_strategy(field),
        field_type: get_field_type(field),
        is_generated: Some(false),
        is_updated_at: Some(field.is_updated_at()),
        documentation: field.documentation().map(|v| v.to_owned()),
    }
}

fn get_field_kind(field: &dml::Field) -> String {
    match field.field_type() {
        dml::FieldType::CompositeType(_) => String::from("object"),
        dml::FieldType::Relation(_) => String::from("object"),
        dml::FieldType::Enum(_) => String::from("enum"),
        dml::FieldType::Scalar(_, _) => String::from("scalar"),
        dml::FieldType::Unsupported(_) => String::from("unsupported"),
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

fn get_field_type(field: &dml::Field) -> String {
    match &field.field_type() {
        dml::FieldType::CompositeType(t) => t.clone(),
        dml::FieldType::Relation(relation_info) => relation_info.referenced_model.clone(),
        dml::FieldType::Enum(t) => t.clone(),
        dml::FieldType::Unsupported(t) => t.clone(),
        dml::FieldType::Scalar(t, _) => type_to_string(t),
    }
}

fn type_to_string(scalar: &ScalarType) -> String {
    scalar.to_string()
}

fn get_relation_name(field: &dml::Field) -> Option<String> {
    match &field {
        dml::Field::RelationField(rf) => Some(rf.relation_info.name.clone()),
        _ => None,
    }
}

fn get_relation_from_fields(field: &dml::Field) -> Option<Vec<String>> {
    match &field {
        dml::Field::RelationField(rf) => Some(rf.relation_info.fields.clone()),
        _ => None,
    }
}

fn get_relation_to_fields(field: &dml::Field) -> Option<Vec<String>> {
    match &field {
        dml::Field::RelationField(rf) => Some(rf.relation_info.references.clone()),
        _ => None,
    }
}

fn get_relation_delete_strategy(field: &dml::Field) -> Option<String> {
    match &field {
        dml::Field::RelationField(rf) => rf.relation_info.on_delete.map(|ri| ri.to_string()),
        _ => None,
    }
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
        ];

        for test_case in test_cases {
            println!("TESTING: {test_case}");

            let datamodel_string = load_from_file(format!("{test_case}.prisma").as_str());
            let dmmf_string = render_to_dmmf(&datamodel_string);

            assert_eq_json(
                &dmmf_string,
                &load_from_file(format!("{test_case}.json").as_str()),
                test_case,
            );
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
