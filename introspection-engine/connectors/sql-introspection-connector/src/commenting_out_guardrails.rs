use crate::warnings::{warning_enum_values_with_empty_names, EnumAndValue};
use introspection_connector::Warning;
use psl::dml::Datamodel;

pub(crate) fn commenting_out_guardrails(datamodel: &mut Datamodel) -> Vec<Warning> {
    let mut warnings = vec![];

    let enum_values_with_empty_names = empty_enum_values(datamodel);

    if !enum_values_with_empty_names.is_empty() {
        warnings.push(warning_enum_values_with_empty_names(&enum_values_with_empty_names))
    }

    warnings
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
