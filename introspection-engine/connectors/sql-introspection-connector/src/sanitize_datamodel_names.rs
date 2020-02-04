use datamodel::{Datamodel, FieldType};
use regex::Regex;
use std::collections::HashMap;

pub fn sanitize_datamodel_names(mut datamodel: Datamodel) -> Datamodel {
    let mut enum_renames = HashMap::new();

    for model in &mut datamodel.models {
        let (sanitized_model_name, model_db_name) = sanitize_name(model.name.clone());

        for field in &mut model.fields {
            let (sanitized_field_name, field_db_name) = sanitize_name(field.name.clone());

            if let FieldType::Relation(info) = &mut field.field_type {
                info.name = sanitize_name(info.name.clone()).0;
                info.to = sanitize_name(info.to.clone()).0;
                info.to_fields = info
                    .to_fields
                    .iter()
                    .map(|f: &std::string::String| sanitize_name(f.clone()).0)
                    .collect();
            }

            if let FieldType::Enum(enum_name) = &mut field.field_type {
                let (sanitized_enum_name, enum_db_name) = if *enum_name == format!("{}_{}", model.name, field.name) {
                    //MySql
                    if model_db_name.is_none() && field_db_name.is_none() {
                        (enum_name.clone(), None)
                    } else {
                        (
                            format!("{}_{}", sanitized_model_name, sanitized_field_name),
                            Some(enum_name.clone()),
                        )
                    }
                } else {
                    sanitize_name(enum_name.clone())
                };

                if let Some(old_name) = enum_db_name {
                    enum_renames.insert(old_name.clone(), (sanitized_enum_name.clone(), Some(old_name.clone())));
                };

                *enum_name = sanitized_enum_name;
            }

            field.name = sanitized_field_name;

            if field.database_names.is_empty() {
                field.database_names = field_db_name.map(|db| vec![db]).unwrap_or(vec![]);
            }
        }

        for index in &mut model.indices {
            index.fields = index.fields.iter().map(|f| sanitize_name(f.clone()).0).collect();
        }

        model.name = sanitized_model_name;
        model.database_name = model_db_name;
    }

    for enm in &mut datamodel.enums {
        if let Some((sanitized_enum_name, enum_db_name)) = enum_renames.get(&enm.name) {
            enm.name = sanitized_enum_name.to_owned();
            enm.database_name = enum_db_name.to_owned();
        } else {
            let (sanitized_enum_name, enum_db_name) = sanitize_name(enm.name.clone());
            enm.name = sanitized_enum_name.to_owned();
            enm.database_name = enum_db_name.to_owned();
        }
    }

    // todo: do a pass over all modified names and deduplicate them

    datamodel
}

fn sanitize_name(name: String) -> (String, Option<String>) {
    let re_start = Regex::new("^[^a-zA-Z]+").unwrap();
    let re = Regex::new("[^_a-zA-Z0-9]").unwrap();
    let needs_sanitation = re_start.is_match(name.as_str()) || re.is_match(name.as_str());

    if needs_sanitation {
        let start_cleaned: String = re_start.replace_all(name.as_str(), "").parse().unwrap();
        (re.replace_all(start_cleaned.as_str(), "_").parse().unwrap(), Some(name))
    } else {
        (name, None)
    }
}
