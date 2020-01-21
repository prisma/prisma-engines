use super::*;
use crate::ast::Span;
use crate::common::FromStrAndSpan;
use crate::common::ScalarType;
use crate::{dml, ValueGenerator};
use chrono::{DateTime, Utc};
use datamodel_connector::scalars::ScalarValue;

pub fn parse_from_dmmf(dmmf: &str) -> dml::Datamodel {
    let parsed_dmmf = serde_json::from_str::<Datamodel>(&dmmf).expect("Failed to parse JSON");
    schema_from_dmmf(&parsed_dmmf)
}

pub fn schema_from_dmmf(schema: &Datamodel) -> dml::Datamodel {
    let mut datamodel = dml::Datamodel {
        models: vec![],
        enums: vec![],
    };

    for model in &schema.models {
        datamodel.add_model(model_from_dmmf(&model));
    }

    for enum_model in &schema.enums {
        datamodel.add_enum(enum_from_dmmf(&enum_model));
    }

    datamodel
}

//todo
fn model_from_dmmf(model: &Model) -> dml::Model {
    dml::Model {
        name: model.name.clone(),
        database_name: model.db_name.clone(),
        is_embedded: model.is_embedded,
        fields: model.fields.iter().map(&field_from_dmmf).collect(),
        indices: vec![],
        id_fields: model.id_fields.clone(),
        documentation: model.documentation.clone(),
        is_generated: model.is_generated.unwrap_or(false),
    }
}

//todo
fn enum_from_dmmf(en: &Enum) -> dml::Enum {
    dml::Enum {
        name: en.name.clone(),
        values: en.values.clone(),
        database_name: en.db_name.clone(),
        documentation: en.documentation.clone(),
    }
}

fn field_from_dmmf(field: &Field) -> dml::Field {
    let field_type = get_field_type(field);
    let default_value = default_value_from_serde(&field.default, &field_type);

    // TODO: Id details?
    let id_info = match &field.is_id {
        true => Some(dml::IdInfo {
            strategy: dml::IdStrategy::Auto,
            sequence: None,
        }),
        false => None,
    };

    //Todo
    dml::Field {
        name: field.name.clone(),
        arity: get_field_arity(field.is_required, field.is_list),
        database_names: field.db_names.clone(),
        field_type,
        default_value,
        id_info,
        is_unique: field.is_unique,
        is_generated: field.is_generated.unwrap_or(false),
        is_updated_at: field.is_updated_at.unwrap_or(false),
        documentation: field.documentation.clone(),
    }
}

fn default_value_from_serde(
    container: &Option<serde_json::Value>,
    field_type: &dml::FieldType,
) -> Option<dml::DefaultValue> {
    match (container, field_type) {
        // Scalar.
        (Some(value), dml::FieldType::Base(scalar_type)) => match (value, scalar_type) {
            // Function.
            (serde_json::Value::Object(_), _) => {
                let func = serde_json::from_value::<Function>(value.clone()).expect("Failed to parse function JSON");
                let vg = function_from_dmmf(&func, *scalar_type);
                Some(dml::DefaultValue::Expression(vg))
            }
            (json, scalar_type) => {
                let sv = parse_as_scalar_value(&json, &scalar_type);
                Some(dml::DefaultValue::Single(sv))
            }
        },
        // Enum.
        (Some(value), dml::FieldType::Enum(_)) => Some(dml::DefaultValue::Single(dml::ScalarValue::ConstantLiteral(
            String::from(value.as_str().unwrap()),
        ))),
        (Some(_), _) => panic!("Fields with non-scalar type cannot have default value"),
        _ => None,
    }
}

fn parse_as_scalar_value(json: &serde_json::Value, scalar_type: &ScalarType) -> ScalarValue {
    match (json, scalar_type) {
        (serde_json::Value::Bool(val), ScalarType::Boolean) => dml::ScalarValue::Boolean(*val),
        (serde_json::Value::String(val), ScalarType::String) => dml::ScalarValue::String(String::from(val.as_str())),
        (serde_json::Value::Number(val), ScalarType::Float) => dml::ScalarValue::Float(val.as_f64().unwrap() as f32),
        (serde_json::Value::Number(val), ScalarType::Int) => dml::ScalarValue::Int(val.as_i64().unwrap() as i32),
        (serde_json::Value::Number(val), ScalarType::Decimal) => {
            dml::ScalarValue::Decimal(val.as_f64().unwrap() as f32)
        }
        (serde_json::Value::String(val), ScalarType::DateTime) => {
            dml::ScalarValue::DateTime(String::from(val.as_str()).parse::<DateTime<Utc>>().unwrap())
        }
        _ => panic!(
            "Invalid type/value combination for scalar value. Type: {:?}, Value: {}",
            &scalar_type, &json
        ),
    }
}

fn type_from_string(scalar: &str) -> ScalarType {
    ScalarType::from_str(scalar).unwrap()
}

fn function_from_dmmf(func: &Function, expected_type: ScalarType) -> dml::ValueGenerator {
    if !func.args.is_empty() {
        panic!("Function argument deserialization is not supported with DMMF. There are no type annotations yet, so it's not clear which is meant.");
    }

    if func.return_type != expected_type.to_string() {
        panic!(
            "Type missmatch during deserialization. Expected: {}, but got: {}.",
            expected_type.to_string(),
            func.return_type
        );
    }

    let vg = ValueGenerator::new(func.name.clone(), vec![]).unwrap();
    vg
}

fn get_on_delete_strategy(strategy: &Option<String>) -> dml::OnDeleteStrategy {
    match strategy {
        Some(val) => dml::OnDeleteStrategy::from_str_and_span(&val, Span::empty()).unwrap(),
        None => dml::OnDeleteStrategy::None,
    }
}

fn get_field_type(field: &Field) -> dml::FieldType {
    match &field.kind as &str {
        "object" => dml::FieldType::Relation(dml::RelationInfo {
            to: field.field_type.clone(),
            to_fields: field.relation_to_fields.clone().unwrap_or_default(),
            name: field.relation_name.clone().unwrap_or(String::new()),
            on_delete: get_on_delete_strategy(&field.relation_on_delete),
        }),
        "enum" => dml::FieldType::Enum(field.field_type.clone()),
        "scalar" => dml::FieldType::Base(type_from_string(&field.field_type)),
        _ => panic!(format!("Unknown field kind {}.", &field.kind)),
    }
}

fn get_field_arity(is_required: bool, is_list: bool) -> dml::FieldArity {
    match (is_required, is_list) {
        (true, true) => dml::FieldArity::List,
        (false, true) => dml::FieldArity::List,
        (true, false) => dml::FieldArity::Required,
        (false, false) => dml::FieldArity::Optional,
    }
}
