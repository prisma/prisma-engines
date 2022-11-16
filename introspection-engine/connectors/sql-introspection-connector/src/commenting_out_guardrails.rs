use crate::warnings::{
    warning_enum_values_with_empty_names, warning_fields_with_empty_names, EnumAndValue, ModelAndField,
};
use introspection_connector::Warning;
use psl::dml::Datamodel;

pub(crate) fn commenting_out_guardrails(datamodel: &mut Datamodel) -> Vec<Warning> {
    let mut warnings = vec![];

    // order matters...
    let fields_with_empty_names = fields_with_empty_names(datamodel);
    let enum_values_with_empty_names = empty_enum_values(datamodel);

    if !fields_with_empty_names.is_empty() {
        warnings.push(warning_fields_with_empty_names(&fields_with_empty_names))
    }

    if !enum_values_with_empty_names.is_empty() {
        warnings.push(warning_enum_values_with_empty_names(&enum_values_with_empty_names))
    }

    warnings
}

fn fields_with_empty_names(datamodel: &mut Datamodel) -> Vec<ModelAndField> {
    let mut fields_with_empty_names = vec![];

    for model in datamodel.models_mut() {
        let model_name = model.name.clone();

        for field in model.scalar_fields_mut() {
            if field.name.is_empty() {
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

    fields_with_empty_names
}

fn empty_enum_values(datamodel: &mut Datamodel) -> Vec<EnumAndValue> {
    let mut enum_values_with_empty_names = vec![];

    for enm in datamodel.enums_mut() {
        let enum_name = enm.name.clone();

        for enum_value in enm.values_mut() {
            let name = match &enum_value.database_name {
                Some(name) => name,
                None => continue,
            };

            if !enum_value.name.is_empty() {
                continue;
            }

            enum_value.name = name.clone();
            enum_value.commented_out = true;
            enum_values_with_empty_names.push(EnumAndValue::new(&enum_name, &enum_value.name))
        }
    }

    enum_values_with_empty_names
}
