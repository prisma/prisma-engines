use datamodel::{Datamodel, DefaultValue, FieldType, ScalarValue};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

pub fn sanitize_datamodel_names(datamodel: &mut Datamodel) {
    let mut enum_renames = HashMap::new();

    for model in &mut datamodel.models {
        let (sanitized_model_name, model_db_name) = sanitize_name(model.name.clone());

        for field in &mut model.fields {
            let (sanitized_field_name, field_db_name) = sanitize_name(field.name.clone());
            let id_field_option = model.id_fields.iter_mut().find(|name| **name == field.name);
            let mut no_db_name_because_field_is_virtual = false;

            match &mut field.field_type {
                FieldType::Relation(info) => {
                    info.name = sanitize_name(info.name.clone()).0;
                    info.to = sanitize_name(info.to.clone()).0;
                    info.to_fields = info
                        .to_fields
                        .iter()
                        .map(|f: &std::string::String| sanitize_name(f.clone()).0)
                        .collect();

                    no_db_name_because_field_is_virtual = info.to_fields.is_empty();
                }
                FieldType::Enum(enum_name) => {
                    let (sanitized_enum_name, enum_db_name) = if *enum_name == format!("{}_{}", model.name, field.name)
                    {
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

                    if let Some(DefaultValue::Single(ScalarValue::ConstantLiteral(name))) = &mut field.default_value {
                        let (sanitized_value, _) = sanitize_name(name.to_string());
                        *name = sanitized_value;
                    };
                }
                _ => (),
            }

            field.name = sanitized_field_name.clone();
            id_field_option.map(|id_field| *id_field = sanitized_field_name.clone());

            if field.database_names.is_empty() && !no_db_name_because_field_is_virtual {
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

        for enum_value in &mut enm.values {
            let (sanitized_name, db_name) = sanitize_name(enum_value.name.clone());
            enum_value.name = sanitized_name;
            enum_value.database_name = db_name;
        }
    }
}

static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new("^[^a-zA-Z]+").unwrap());

static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

//todo this is now widely used, we can make this smarter at some point
//ideas:
// numbers only -> spell out first digit?   100 -> one00
// Only invalid characters??
// Underscore at start
fn sanitize_name(name: String) -> (String, Option<String>) {
    let needs_sanitation = RE_START.is_match(name.as_str()) || RE.is_match(name.as_str());

    if needs_sanitation {
        let start_cleaned: String = RE_START.replace_all(name.as_str(), "").parse().unwrap();
        (RE.replace_all(start_cleaned.as_str(), "_").parse().unwrap(), Some(name))
    } else {
        (name, None)
    }
}
