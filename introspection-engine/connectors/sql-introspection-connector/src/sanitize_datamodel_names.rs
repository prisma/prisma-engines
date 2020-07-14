use datamodel::{Datamodel, DefaultValue, Field, FieldType, WithName};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use std::collections::HashMap;

static EMPTY_ENUM_PLACEHOLDER: &'static str = "EMPTY_ENUM_VALUE";
static EMPTY_STRING: &'static str = "";

//todo sanitizing might need to be adjusted to also change the fields in the RelationInfo
pub fn sanitize_datamodel_names(datamodel: &mut Datamodel) {
    let mut enum_renames = HashMap::new();

    for model in datamodel.models_mut() {
        let (sanitized_model_name, model_db_name) = sanitize_name(model.name.clone());
        let model_name = model.name.clone();
        sanitize_names(&mut model.id_fields);

        for field in model.fields_mut() {
            let (sanitized_field_name, field_db_name) = sanitize_name(field.name().to_string());

            match field {
                Field::RelationField(rf) => {
                    let info = &mut rf.relation_info;
                    info.name = sanitize_name(info.name.clone()).0;
                    info.to = sanitize_name(info.to.clone()).0;
                    sanitize_names(&mut info.to_fields);
                    sanitize_names(&mut info.fields);
                }
                Field::ScalarField(sf) => {
                    if let FieldType::Enum(enum_name) = &sf.field_type {
                        let (sanitized_enum_name, enum_db_name) = if *enum_name == format!("{}_{}", model_name, sf.name)
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
                            enum_renames
                                .insert(old_name.clone(), (sanitized_enum_name.clone(), Some(old_name.clone())));
                        };

                        sf.field_type = FieldType::Enum(sanitized_enum_name);

                        if let Some(DefaultValue::Single(PrismaValue::Enum(value))) = &mut sf.default_value {
                            if EMPTY_STRING == value {
                                *value = EMPTY_ENUM_PLACEHOLDER.to_string();
                            } else {
                                let (sanitized_value, _) = sanitize_name(value.to_string());

                                sf.default_value = Some(match sanitized_value {
                                    x if x == EMPTY_STRING => DefaultValue::new_db_generated(),
                                    _ => DefaultValue::Single(PrismaValue::Enum(sanitized_value)),
                                });
                            }
                        };
                    }

                    if sf.database_name.is_none() {
                        sf.database_name = field_db_name;
                    }
                }
            }
            field.set_name(&sanitized_field_name);
        }

        for index in &mut model.indices {
            sanitize_names(&mut index.fields);
        }

        model.name = sanitized_model_name;
        model.database_name = model_db_name;
    }

    for enm in datamodel.enums_mut() {
        if let Some((sanitized_enum_name, enum_db_name)) = enum_renames.get(&enm.name) {
            enm.name = sanitized_enum_name.to_owned();
            enm.database_name = enum_db_name.to_owned();
        } else {
            let (sanitized_enum_name, enum_db_name) = sanitize_name(enm.name.clone());
            enm.name = sanitized_enum_name.to_owned();
            enm.database_name = enum_db_name.to_owned();
        }

        for enum_value in enm.values_mut() {
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

fn sanitize_names(names: &mut [String]) {
    names
        .iter_mut()
        .map(|f| *f = sanitize_name(f.to_string()).0)
        .for_each(drop);
}
