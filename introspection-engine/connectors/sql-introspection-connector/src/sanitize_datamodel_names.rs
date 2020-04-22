use datamodel::{Datamodel, DefaultValue, FieldType, ValueGenerator};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use std::collections::HashMap;

static EMPTY_ENUM_PLACEHOLDER: &'static str = "EMPTY_ENUM_VALUE";
static EMPTY_STRING: &'static str = "";

pub fn sanitize_datamodel_names(datamodel: &mut Datamodel) {
    let mut enum_renames = HashMap::new();

    for model in &mut datamodel.models {
        let (sanitized_model_name, model_db_name) = sanitize_name(model.name.clone());

        for field in &mut model.fields {
            let (sanitized_field_name, field_db_name) = sanitize_name(field.name.clone());
            let id_field_option = model.id_fields.iter_mut().find(|name| **name == field.name);

            match &mut field.field_type {
                FieldType::Relation(info) => {
                    info.name = sanitize_name(info.name.clone()).0;
                    info.to = sanitize_name(info.to.clone()).0;
                    info.to_fields = info
                        .to_fields
                        .iter()
                        .map(|f: &std::string::String| sanitize_name(f.clone()).0)
                        .collect();
                    info.fields = info
                        .fields
                        .iter()
                        .map(|f: &std::string::String| sanitize_name(f.clone()).0)
                        .collect();
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

                    if let Some(DefaultValue::Single(PrismaValue::Enum(value))) = &mut field.default_value {
                        if EMPTY_STRING == value {
                            *value = EMPTY_ENUM_PLACEHOLDER.to_string();
                        } else {
                            let (sanitized_value, _) = sanitize_name(value.to_string());

                            field.default_value = if sanitized_value == EMPTY_STRING.to_string() {
                                Some(DefaultValue::Expression(ValueGenerator::new_dbgenerated()))
                            } else {
                                Some(DefaultValue::Single(PrismaValue::Enum(sanitized_value)))
                            };
                        }
                    };

                    if field.database_name.is_none() {
                        field.database_name = field_db_name;
                    }
                }
                _ => {
                    if field.database_name.is_none() {
                        field.database_name = field_db_name;
                    }
                }
            }

            field.name = sanitized_field_name.clone();
            id_field_option.map(|id_field| *id_field = sanitized_field_name.clone());
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
            if &enum_value.name == EMPTY_STRING {
                enum_value.name = EMPTY_ENUM_PLACEHOLDER.to_string();
                enum_value.database_name = Some(EMPTY_STRING.to_string());
            } else {
                let (sanitized_name, db_name) = sanitize_name(enum_value.name.clone());
                enum_value.name = sanitized_name;
                enum_value.database_name = db_name;
            }
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
