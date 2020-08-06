use super::*;
use crate::common::ScalarType;
use crate::{dml, IndexType};
use prisma_value::PrismaValue;
use rust_decimal::prelude::ToPrimitive;

pub fn render_to_dmmf(schema: &dml::Datamodel) -> String {
    let dmmf = schema_to_dmmf(schema);
    serde_json::to_string_pretty(&dmmf).expect("Failed to render JSON")
}

pub fn render_to_dmmf_value(schema: &dml::Datamodel) -> serde_json::Value {
    let dmmf = schema_to_dmmf(schema);
    serde_json::to_value(&dmmf).expect("Failed to render JSON")
}

fn schema_to_dmmf(schema: &dml::Datamodel) -> Datamodel {
    let mut datamodel = Datamodel {
        models: vec![],
        enums: vec![],
    };

    for enum_model in schema.enums() {
        datamodel.enums.push(enum_to_dmmf(&enum_model));
    }

    for model in schema.models() {
        datamodel.models.push(model_to_dmmf(&model));
    }

    datamodel
}

fn enum_to_dmmf(en: &dml::Enum) -> Enum {
    let mut enm = Enum {
        name: en.name.clone(),
        values: vec![],
        db_name: en.database_name.clone(),
        documentation: en.documentation.clone(),
    };

    for enum_value in en.values() {
        enm.values.push(enum_value_to_dmmf(enum_value));
    }

    enm
}

fn enum_value_to_dmmf(en: &dml::EnumValue) -> EnumValue {
    EnumValue {
        name: en.name.clone(),
        db_name: en.database_name.clone(),
    }
}

fn model_to_dmmf(model: &dml::Model) -> Model {
    Model {
        name: model.name.clone(),
        db_name: model.database_name.clone(),
        is_embedded: model.is_embedded,
        fields: model.fields().map(|f| field_to_dmmf(model, f)).collect(),
        is_generated: Some(model.is_generated),
        documentation: model.documentation.clone(),
        id_fields: model.id_fields.clone(),
        unique_fields: model
            .indices
            .iter()
            .filter_map(|i| {
                if i.tpe == IndexType::Unique {
                    Some(i.fields.clone())
                } else {
                    None
                }
            })
            .collect(),
        unique_indexes: model
            .indices
            .iter()
            .filter_map(|i| {
                if i.tpe == IndexType::Unique {
                    Some(UniqueIndex {
                        name: i.name.clone(),
                        fields: i.fields.clone(),
                    })
                } else {
                    None
                }
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
        kind: get_field_kind(field),
        is_required: *field.arity() == dml::FieldArity::Required,
        is_list: *field.arity() == dml::FieldArity::List,
        is_id: field.is_id(),
        is_read_only: a_relation_field_is_based_on_this_field,
        has_default_value: field.default_value().is_some(),
        default: default_value_to_serde(&field.default_value().cloned()),
        is_unique: field.is_unique(),
        relation_name: get_relation_name(field),
        relation_from_fields: get_relation_from_fields(field),
        relation_to_fields: get_relation_to_fields(field),
        relation_on_delete: get_relation_delete_strategy(field),
        field_type: get_field_type(field),
        is_generated: Some(field.is_generated()),
        is_updated_at: Some(field.is_updated_at()),
        documentation: field.documentation().map(|v| v.to_owned()),
    }
}

fn get_field_kind(field: &dml::Field) -> String {
    match field.field_type() {
        dml::FieldType::Relation(_) => String::from("object"),
        dml::FieldType::Enum(_) => String::from("enum"),
        dml::FieldType::Base(_, _) => String::from("scalar"),
        tpe => unimplemented!("DMMF does not support field type {:?}", tpe),
    }
}

fn default_value_to_serde(dv_opt: &Option<dml::DefaultValue>) -> Option<serde_json::Value> {
    dv_opt.as_ref().map(|dv| match dv {
        dml::DefaultValue::Single(value) => prisma_value_to_serde(&value.clone()),
        dml::DefaultValue::Expression(vg) => function_to_serde(&vg.name, &vg.args),
    })
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
        PrismaValue::DateTime(val) => serde_json::Value::String(val.to_rfc3339()),
        PrismaValue::Null(_) => serde_json::Value::Null,
        PrismaValue::Uuid(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::Json(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::List(value_vec) => {
            serde_json::Value::Array(value_vec.iter().map(|pv| prisma_value_to_serde(pv)).collect())
        }
    }
}

fn function_to_serde(name: &str, args: &[PrismaValue]) -> serde_json::Value {
    let func = Function {
        name: String::from(name),
        args: args.iter().map(|arg| prisma_value_to_serde(arg)).collect(),
    };

    serde_json::to_value(&func).expect("Failed to render function JSON")
}

fn get_field_type(field: &dml::Field) -> String {
    match &field.field_type() {
        dml::FieldType::Relation(relation_info) => relation_info.to.clone(),
        dml::FieldType::Enum(t) => t.clone(),
        dml::FieldType::Unsupported(t) => t.clone(),
        dml::FieldType::Base(t, _) => type_to_string(t),
        dml::FieldType::NativeType(t, _) => type_to_string(t),
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
        dml::Field::RelationField(rf) => Some(rf.relation_info.to_fields.clone()),
        _ => None,
    }
}

fn get_relation_delete_strategy(field: &dml::Field) -> Option<String> {
    match &field {
        dml::Field::RelationField(rf) => Some(rf.relation_info.on_delete.to_string()),
        _ => None,
    }
}
