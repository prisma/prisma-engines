use crate::{Datamodel, Enum, EnumValue, Field, Function, Model, PrimaryKey, UniqueIndex};
use bigdecimal::ToPrimitive;
use datamodel::{
    dml::{self, CompositeTypeFieldType, FieldType, Ignorable, ScalarType},
    PrismaValue,
};

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
        types: Vec::with_capacity(schema.composite_types.len()),
    };

    for enum_model in schema.enums() {
        datamodel.enums.push(enum_to_dmmf(enum_model));
    }

    for model in schema.models().filter(|model| !model.is_ignored) {
        datamodel.models.push(model_to_dmmf(model));
    }

    for ct in schema.composite_types() {
        datamodel.types.push(composite_type_to_dmmf(ct))
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

fn composite_type_to_dmmf(ct: &dml::CompositeType) -> Model {
    Model {
        name: ct.name.clone(),
        db_name: None,
        fields: ct
            .fields
            .iter()
            .filter(|field| !matches!(&field.r#type, CompositeTypeFieldType::Unsupported(_)))
            .map(composite_type_field_to_dmmf)
            .collect(),
        is_generated: None,
        documentation: None,
        primary_key: None,
        unique_fields: Vec::new(),
        unique_indexes: Vec::new(),
    }
}

fn composite_type_field_to_dmmf(field: &dml::CompositeTypeField) -> Field {
    Field {
        name: field.name.clone(),
        kind: match field.r#type {
            CompositeTypeFieldType::CompositeType(_) => String::from("object"),
            CompositeTypeFieldType::Enum(_) => String::from("enum"),
            CompositeTypeFieldType::Scalar(_, _, _) => String::from("scalar"),
            CompositeTypeFieldType::Unsupported(_) => String::from("unsupported"),
        },
        is_required: field.arity == dml::FieldArity::Required || field.arity == dml::FieldArity::List,
        is_list: field.arity == dml::FieldArity::List,
        is_id: false,
        is_read_only: false,
        has_default_value: field.default_value.is_some(),
        default: default_value_to_serde(&field.default_value),
        is_unique: false,
        relation_name: None,
        relation_from_fields: None,
        relation_to_fields: None,
        relation_on_delete: None,
        field_type: match &field.r#type {
            CompositeTypeFieldType::CompositeType(t) => t.clone(),
            CompositeTypeFieldType::Enum(t) => t.clone(),
            CompositeTypeFieldType::Unsupported(t) => t.clone(),
            CompositeTypeFieldType::Scalar(t, _, _) => type_to_string(t),
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
                (i.is_unique() && !i.defined_on_field).then(|| i.fields.clone().into_iter().map(|f| f.name).collect())
            })
            .collect(),
        unique_indexes: model
            .indices
            .iter()
            .filter_map(|i| {
                (i.is_unique() && !i.defined_on_field).then(|| UniqueIndex {
                    name: i.name.clone(),
                    //TODO(extended indices) add field options here
                    fields: i.fields.clone().into_iter().map(|f| f.name).collect(),
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
        kind: get_field_kind(field),
        is_required: *field.arity() == dml::FieldArity::Required || *field.arity() == dml::FieldArity::List,
        is_list: *field.arity() == dml::FieldArity::List,
        is_id: model.field_is_primary(field.name()),
        is_read_only: a_relation_field_is_based_on_this_field,
        has_default_value: field.default_value().is_some(),
        default: default_value_to_serde(&field.default_value().cloned()),
        is_unique: model.field_is_unique(field.name()),
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
        dml::FieldType::CompositeType(_) => String::from("object"),
        dml::FieldType::Relation(_) => String::from("object"),
        dml::FieldType::Enum(_) => String::from("enum"),
        dml::FieldType::Scalar(_, _, _) => String::from("scalar"),
        dml::FieldType::Unsupported(_) => String::from("unsupported"),
    }
}

fn default_value_to_serde(dv_opt: &Option<dml::DefaultValue>) -> Option<serde_json::Value> {
    dv_opt.as_ref().map(|dv| match dv.kind() {
        dml::DefaultKind::Single(value) => prisma_value_to_serde(&value.clone()),
        dml::DefaultKind::Expression(vg) => function_to_serde(vg.name(), vg.args()),
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
        PrismaValue::BigInt(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::DateTime(val) => serde_json::Value::String(val.to_rfc3339()),
        PrismaValue::Null => serde_json::Value::Null,
        PrismaValue::Uuid(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::Json(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::Xml(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::List(value_vec) => serde_json::Value::Array(value_vec.iter().map(prisma_value_to_serde).collect()),
        PrismaValue::Bytes(b) => serde_json::Value::String(datamodel::prisma_value::encode_bytes(b)),
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
        dml::FieldType::Relation(relation_info) => relation_info.to.clone(),
        dml::FieldType::Enum(t) => t.clone(),
        dml::FieldType::Unsupported(t) => t.clone(),
        dml::FieldType::Scalar(t, _, _) => type_to_string(t),
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
