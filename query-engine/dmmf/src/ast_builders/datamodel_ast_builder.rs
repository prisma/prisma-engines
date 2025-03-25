use crate::serialization_ast::{
    datamodel_ast::{Datamodel, Enum, EnumValue, Field, Function, Model, PrimaryKey, UniqueIndex},
    Index, IndexField, IndexType,
};
use bigdecimal::ToPrimitive;
use itertools::{Either, Itertools};
use psl::{
    parser_database::{walkers, ScalarFieldType},
    schema_ast::ast::WithDocumentation,
};
use query_structure::{dml_default_kind, encode_bytes, DefaultKind, FieldArity, PrismaValue};

pub(crate) fn schema_to_dmmf(schema: &psl::ValidatedSchema) -> Datamodel {
    let mut datamodel = Datamodel {
        models: Vec::with_capacity(schema.db.models_count()),
        enums: Vec::with_capacity(schema.db.enums_count()),
        types: Vec::new(),
        indexes: Vec::new(),
    };

    for enum_model in schema.db.walk_enums() {
        datamodel.enums.push(enum_to_dmmf(enum_model));
    }

    for model in schema
        .db
        .walk_models()
        .filter(|model| !model.is_ignored())
        .chain(schema.db.walk_views().filter(|view| !view.is_ignored()))
    {
        datamodel.models.push(model_to_dmmf(model));
        datamodel.indexes.extend(model_indexes_to_dmmf(model));
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
        documentation: en.documentation().map(ToOwned::to_owned),
    }
}

fn composite_type_to_dmmf(ct: walkers::CompositeTypeWalker<'_>) -> Model {
    Model {
        name: ct.name().to_owned(),
        db_name: None,
        schema: None,
        fields: ct
            .fields()
            .filter(|field| !matches!(field.r#type(), ScalarFieldType::Unsupported(_)))
            .map(composite_type_field_to_dmmf)
            .collect(),
        is_generated: None,
        documentation: ct.ast_composite_type().documentation().map(ToOwned::to_owned),
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
        is_required: field.arity() == FieldArity::Required || field.arity() == FieldArity::List,
        is_list: field.arity() == FieldArity::List,
        is_id: false,
        is_read_only: false,
        has_default_value: field.default_value().is_some(),
        native_type: field
            .raw_native_type()
            .map(|(_, name, args, ..)| (name.to_string(), args.to_vec())),
        default: field
            .default_value()
            .map(|dv| default_value_to_serde(&dml_default_kind(dv, field.scalar_type()))),
        is_unique: false,
        relation_name: None,
        relation_from_fields: None,
        relation_to_fields: None,
        relation_on_delete: None,
        relation_on_update: None,
        field_type: match field.r#type() {
            ScalarFieldType::CompositeType(ct) => field.walk(ct).name().to_owned(),
            ScalarFieldType::Enum(enm) => field.walk(enm).name().to_owned(),
            ScalarFieldType::BuiltInScalar(st) => st.as_str().to_owned(),
            ScalarFieldType::Unsupported(_) => unreachable!(),
        },
        is_generated: None,
        is_updated_at: None,
        documentation: field.documentation().map(ToOwned::to_owned),
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
        schema: model.schema().map(|(s, _)| s.to_owned()),
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
            .filter(|&i| i.is_unique() && !i.is_defined_on_field())
            .map(|i| i.fields().map(|f| f.name().to_owned()).collect())
            .collect(),
        unique_indexes: model
            .indexes()
            .filter(|&i| i.is_unique() && !i.is_defined_on_field())
            .map(|i| UniqueIndex {
                name: i.name().map(ToOwned::to_owned),
                fields: i.fields().map(|f| f.name().to_owned()).collect(),
            })
            .collect(),
    }
}

fn should_skip_model_field(field: &walkers::FieldWalker<'_>) -> bool {
    match field.refine_known() {
        walkers::RefinedFieldWalker::Scalar(f) => f.is_ignored() || f.is_unsupported(),
        walkers::RefinedFieldWalker::Relation(f) => f.is_ignored(),
    }
}

fn field_to_dmmf(field: walkers::FieldWalker<'_>) -> Field {
    match field.refine_known() {
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
        is_required: matches!(ast_field.arity, FieldArity::Required | FieldArity::List),
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
        native_type: field
            .raw_native_type()
            .map(|(_, name, args, ..)| (name.to_string(), args.to_vec())),
        default: field
            .default_value()
            .map(|dv| default_value_to_serde(&dml_default_kind(dv.value(), field.scalar_type()))),
        relation_name: None,
        relation_from_fields: None,
        relation_to_fields: None,
        relation_on_delete: None,
        relation_on_update: None,
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
        is_required: matches!(ast_field.arity, FieldArity::Required | FieldArity::List),
        is_unique: false,
        is_id: false,
        is_read_only: false,
        has_default_value: false,
        field_type: field.related_model().name().to_owned(),
        native_type: None,
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
        relation_on_update: field.explicit_on_update().map(|ou| ou.to_string()),
        is_generated: Some(false),
        is_updated_at: Some(false),
        documentation: ast_field.documentation().map(ToOwned::to_owned),
    }
}

fn model_indexes_to_dmmf(model: walkers::ModelWalker<'_>) -> impl Iterator<Item = Index> + '_ {
    model
        .primary_key()
        .into_iter()
        .map(move |pk| Index {
            model: model.name().to_owned(),
            r#type: IndexType::Id,
            is_defined_on_field: pk.is_defined_on_field(),
            name: pk.name().map(ToOwned::to_owned),
            db_name: pk.mapped_name().map(ToOwned::to_owned),
            algorithm: None,
            clustered: pk.clustered(),
            fields: pk
                .scalar_field_attributes()
                .map(scalar_field_attribute_to_dmmf)
                .collect(),
        })
        .chain(model.indexes().map(move |index| {
            Index {
                model: model.name().to_owned(),
                r#type: index.index_type().into(),
                is_defined_on_field: index.is_defined_on_field(),
                name: index.name().map(ToOwned::to_owned),
                db_name: index.mapped_name().map(ToOwned::to_owned),
                algorithm: index.algorithm().map(|alg| alg.to_string()),
                clustered: index.clustered(),
                fields: index
                    .scalar_field_attributes()
                    .map(scalar_field_attribute_to_dmmf)
                    .collect(),
            }
        }))
}

fn scalar_field_attribute_to_dmmf(sfa: walkers::ScalarFieldAttributeWalker<'_>) -> IndexField {
    IndexField {
        name: sfa
            .as_path_to_indexed_field()
            .into_iter()
            .map(|(field_name, _)| field_name.to_owned())
            .join("."),
        sort_order: sfa.sort_order().map(Into::into),
        length: sfa.length(),
        operator_class: sfa.operator_class().map(|oc| match oc.get() {
            Either::Left(oc) => oc.to_string(),
            Either::Right(oc) => oc.to_owned(),
        }),
    }
}

fn default_value_to_serde(dv: &DefaultKind) -> serde_json::Value {
    match dv {
        DefaultKind::Single(value) => prisma_value_to_serde(&value.clone()),
        DefaultKind::Expression(vg) => {
            let args: Vec<_> = vg.args().to_vec();
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
        PrismaValue::Int(val) => serde_json::Value::Number(serde_json::Number::from(*val)),
        PrismaValue::BigInt(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::DateTime(val) => serde_json::Value::String(val.to_rfc3339()),
        PrismaValue::Null => serde_json::Value::Null,
        PrismaValue::Uuid(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::Json(val) => serde_json::Value::String(val.to_string()),
        PrismaValue::List(value_vec) => serde_json::Value::Array(value_vec.iter().map(prisma_value_to_serde).collect()),
        PrismaValue::Bytes(b) => serde_json::Value::String(encode_bytes(b)),
        PrismaValue::Object(pairs) => {
            let mut map = serde_json::Map::with_capacity(pairs.len());
            pairs.iter().for_each(|(key, value)| {
                map.insert(key.clone(), prisma_value_to_serde(value));
            });

            serde_json::Value::Object(map)
        }
        PrismaValue::Placeholder { .. } | PrismaValue::GeneratorCall { .. } => unreachable!(),
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

    const SAMPLES_FOLDER_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test_files");

    #[test]
    fn test_dmmf_rendering() {
        let test_cases = fs::read_dir(SAMPLES_FOLDER_PATH)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.ends_with(".prisma"))
            .map(|name| name.trim_end_matches(".prisma").to_owned());

        for test_case in test_cases {
            println!("TESTING: {test_case}");

            let datamodel_string = load_from_file(format!("{test_case}.prisma").as_str());
            let dmmf_string = render_to_dmmf(&datamodel_string);

            if std::env::var("UPDATE_EXPECT") == Ok("1".into()) {
                save_to_file(&format!("{test_case}.json"), &dmmf_string);
            }

            let expected_json = load_from_file(format!("{test_case}.json").as_str());

            println!("{dmmf_string}");
            assert_eq_json(&dmmf_string, &expected_json, &test_case);
        }
    }

    #[track_caller]
    fn assert_eq_json(a: &str, b: &str, msg: &str) {
        let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
        let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

        assert_eq!(json_a, json_b, "{}", msg);
    }

    fn load_from_file(file: &str) -> String {
        fs::read_to_string(format!("{SAMPLES_FOLDER_PATH}/{file}")).unwrap()
    }

    fn save_to_file(file: &str, content: &str) {
        fs::write(format!("{SAMPLES_FOLDER_PATH}/{file}"), content).unwrap();
    }
}
