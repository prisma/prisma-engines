use datamodel::{Datamodel, FieldArity, FieldType, RelationInfo};
use introspection_connector::Warning;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct Model {
    model: String,
}

#[derive(Serialize, Debug)]
struct ModelAndField {
    model: String,
    field: String,
}

#[derive(Serialize, Debug)]
struct ModelAndFieldType {
    model: String,
    field: String,
    tpe: String,
}

#[derive(Serialize, Debug)]
struct EnumAndValue {
    enm: String,
    value: String,
}

pub fn commenting_out_guardrails(datamodel: &mut Datamodel) -> Vec<Warning> {
    let mut models_without_identifiers = vec![];
    let mut fields_with_empty_names = vec![];
    let mut enum_values_with_empty_names = vec![];
    let mut unsupported_types = vec![];

    // find models with 1to1 relations
    let mut models_with_one_to_one_relation = vec![];
    for model in &datamodel.models {
        if model.fields.iter().any(|f| match (&f.arity, &f.field_type) {
            (FieldArity::List, _) => false,
            (
                _,
                FieldType::Relation(RelationInfo {
                    to,
                    to_fields: _,
                    name: relation_name,
                    ..
                }),
            ) => {
                let other_model = datamodel.find_model(to).unwrap();
                let other_field = other_model
                    .fields
                    .iter()
                    .find(|f| match &f.field_type {
                        FieldType::Relation(RelationInfo {
                            to: other_to,
                            to_fields: _,
                            name: other_relation_name,
                            ..
                        }) if other_to == &model.name && relation_name == other_relation_name => true,
                        _ => false,
                    })
                    .unwrap();

                match other_field.arity {
                    FieldArity::Optional | FieldArity::Required => true,
                    FieldArity::List => false,
                }
            }
            _ => false,
        }) {
            models_with_one_to_one_relation.push(model.name.clone())
        }
    }

    // fields with an empty name
    for model in &mut datamodel.models {
        for field in &mut model.fields {
            if field.name == "".to_string() {
                field.documentation = Some(
                    "This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*"
                        .to_string(),
                );
                field.name = field.database_names.first().unwrap().to_string();
                field.is_commented_out = true;

                fields_with_empty_names.push(ModelAndField {
                    model: model.name.clone(),
                    field: field.name.clone(),
                })
            }
        }
    }

    //empty enum values
    for enm in &mut datamodel.enums {
        for enum_value in &mut enm.values {
            if let Some(name) = &enum_value.database_name {
                if enum_value.name == "".to_string() {
                    enum_value.name = name.clone();
                    enum_value.commented_out = true;
                    enum_values_with_empty_names.push(EnumAndValue {
                        enm: enm.name.clone(),
                        value: enum_value.name.clone(),
                    })
                }
            }
        }
    }

    // fields with unsupported as datatype
    for model in &mut datamodel.models {
        for field in &mut model.fields {
            if let FieldType::Unsupported(tpe) = &field.field_type {
                field.is_commented_out = true;
                unsupported_types.push(ModelAndFieldType {
                    model: model.name.clone(),
                    field: field.name.clone(),
                    tpe: tpe.clone(),
                })
            }
        }
    }

    // use unsupported types to drop @@id / @@unique /@@index
    for mf in &unsupported_types {
        let model = datamodel.find_model_mut(&mf.model).unwrap();
        model.indices.retain(|i| !i.fields.contains(&mf.field));
        if model.id_fields.contains(&mf.field) {
            model.id_fields = vec![]
        };
    }

    // models without uniques / ids
    for model in &mut datamodel.models {
        if model.id_fields.is_empty()
            && !model
                .fields
                .iter()
                .any(|f| (f.is_id || f.is_unique) && !f.is_commented_out)
            && !model.indices.iter().any(|i| i.is_unique())
            && !models_with_one_to_one_relation.contains(&model.name)
        {
            model.is_commented_out = true;
            model.documentation = Some(
                "The underlying table does not contain a unique identifier and can therefore currently not be handled."
                    .to_string(),
            );
            models_without_identifiers.push(Model {
                model: model.name.clone(),
            })
        }
    }

    // remove their backrelations
    for model_without_identifier in &models_without_identifiers {
        for model in &mut datamodel.models {
            model
                .fields
                .retain(|f| !f.points_to_model(model_without_identifier.model.as_ref()));
        }
    }

    let mut warnings = vec![];

    if !models_without_identifiers.is_empty() {
        warnings.push(Warning {
            code: 1,
            message: "These models do not have a unique identifier or id and are therefore commented out.".into(),
            affected: serde_json::to_value(&models_without_identifiers).unwrap(),
        })
    }

    if !fields_with_empty_names.is_empty() {
        warnings.push(Warning {
            code: 2,
            message: "These fields were commented out because of invalid names. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]*."
                .into(),
            affected: serde_json::to_value(&fields_with_empty_names).unwrap(),
        })
    }

    if !unsupported_types.is_empty() {
        warnings.push(Warning {
            code: 3,
            message: "These fields were commented out because we currently do not support their types.".into(),
            affected: serde_json::to_value(&unsupported_types).unwrap(),
        })
    }

    if !enum_values_with_empty_names.is_empty() {
        warnings.push(Warning {
            code: 4,
            message: "These enum values were commented out because of invalid names. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]*."
                .into(),
            affected: serde_json::to_value(&enum_values_with_empty_names).unwrap(),
        })
    }

    warnings
}
