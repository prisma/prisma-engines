use crate::warnings::{
    warning_enum_values_with_empty_names, warning_fields_with_empty_names, warning_models_without_identifier,
    warning_unsupported_types, EnumAndValue, Model, ModelAndField, ModelAndFieldAndType,
};
use datamodel::{Datamodel, FieldType};
use introspection_connector::Warning;

pub fn commenting_out_guardrails(datamodel: &mut Datamodel) -> Vec<Warning> {
    let mut models_without_identifiers = vec![];
    let mut fields_with_empty_names = vec![];
    let mut enum_values_with_empty_names = vec![];
    let mut unsupported_types = vec![];

    //todo more stuff to handle when commenting out. (Maybe it is easier to just work on supporting it.)
    // models with empty names?
    // also needs to follow the field references (relations, indexes, ids...)
    // also needs to drop usages of removed enum values

    // fields with an empty name
    for model in datamodel.models_mut() {
        let model_name = model.name.clone();

        for field in model.scalar_fields_mut() {
            if field.name == *"" {
                field.documentation = Some(
                    "This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*"
                        .to_string(),
                );
                field.name = field.database_name.as_ref().unwrap().to_string();
                field.is_commented_out = true;

                fields_with_empty_names.push(ModelAndField::new(&model_name, &field.name))
            }
        }
    }

    //empty enum values
    for enm in datamodel.enums_mut() {
        let enum_name = enm.name.clone();
        for enum_value in enm.values_mut() {
            if let Some(name) = &enum_value.database_name {
                if enum_value.name == *"" {
                    enum_value.name = name.clone();
                    enum_value.commented_out = true;
                    enum_values_with_empty_names.push(EnumAndValue::new(&enum_name, &enum_value.name))
                }
            }
        }
    }

    // fields with unsupported as datatype
    for model in datamodel.models_mut() {
        let model_name = model.name.clone();

        for field in model.scalar_fields_mut() {
            if let FieldType::Unsupported(tpe) = &field.field_type {
                field.is_commented_out = true;
                unsupported_types.push(ModelAndFieldAndType {
                    model: model_name.clone(),
                    field: field.name.clone(),
                    tpe: tpe.clone(),
                })
            }
        }
    }

    // use unsupported types to drop @@id / @@unique /@@index
    for mf in &unsupported_types {
        let model = datamodel.find_model_mut(&mf.model);
        model.indices.retain(|i| !i.fields.contains(&mf.field));
        if model.id_fields.contains(&mf.field) {
            model.id_fields = vec![]
        };
    }

    // models without uniques / ids
    for model in datamodel.models_mut() {
        if model.strict_unique_criterias().is_empty() {
            model.is_commented_out = true;
            model.documentation = Some(
                "The underlying table does not contain a valid unique identifier and can therefore currently not be handled."
                    .to_string(),
            );
            models_without_identifiers.push(Model {
                model: model.name.clone(),
            })
        }
    }

    // remove their backrelations
    for model_without_identifier in &models_without_identifiers {
        for model in datamodel.models_mut() {
            for field in model.relation_fields_mut() {
                if field.points_to_model(&model_without_identifier.model) {
                    field.is_commented_out = true;
                }
            }
        }
    }

    let mut warnings = vec![];

    if !models_without_identifiers.is_empty() {
        warnings.push(warning_models_without_identifier(&models_without_identifiers))
    }

    if !fields_with_empty_names.is_empty() {
        warnings.push(warning_fields_with_empty_names(&fields_with_empty_names))
    }

    if !unsupported_types.is_empty() {
        warnings.push(warning_unsupported_types(&unsupported_types))
    }

    if !enum_values_with_empty_names.is_empty() {
        warnings.push(warning_enum_values_with_empty_names(&enum_values_with_empty_names))
    }

    warnings
}
