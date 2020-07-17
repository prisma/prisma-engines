use datamodel::{
    transform::ast_to_dml::reserved_model_names, Datamodel, DefaultValue, Field, FieldType, Model, WithDatabaseName,
    WithName,
};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use std::collections::HashMap;

static EMPTY_ENUM_PLACEHOLDER: &'static str = "EMPTY_ENUM_VALUE";
static EMPTY_STRING: &'static str = "";

static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new("^[^a-zA-Z]+").unwrap());
static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

// Todo: Sanitizing might need to be adjusted to also change the fields in the RelationInfo
pub fn sanitize_datamodel_names(datamodel: &mut Datamodel, family: &SqlFamily) {
    let mut enum_renames = HashMap::new();

    for model in datamodel.models_mut() {
        let original_model_name = field.name().to_owned();

        rename_denied(model);
        sanitize_name(model);

        let model_name = model.name.clone();
        model.id_fields = sanitize_strings(model.id_fields.as_slice());

        for field in model.fields_mut() {
            let original_field_name = field.name().to_owned();
            sanitize_name(field);

            match field {
                Field::RelationField(rf) => {
                    let info = &mut rf.relation_info;

                    info.name = sanitize_string(&info.name);
                    info.to = sanitize_string(&info.to);

                    info.to_fields = sanitize_strings(&info.to_fields);
                    info.fields = sanitize_strings(&info.fields);
                }

                Field::ScalarField(sf) => {
                    if let FieldType::Enum(enum_name) = &sf.field_type {
                        if
                        let (sanitized_enum_name, enum_db_name) = if *enum_name == format!("{}_{}", model_name, sf.name)
                        {
                            // MySql
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

        // for index in &mut model.indices {
        //     sanitize_names(&mut index.fields);
        }

        // model.name = sanitized_model_name;
        // model.database_name = model_db_name;
    }

    // for enm in datamodel.enums_mut() {
    //     if let Some((sanitized_enum_name, enum_db_name)) = enum_renames.get(&enm.name) {
    //         enm.name = sanitized_enum_name.to_owned();
    //         enm.database_name = enum_db_name.to_owned();
    //     } else {
    //         sanitize_name(&mut enm);
    //         // let (sanitized_enum_name, enum_db_name) = sanitize_name(enm.name.clone());
    //         // enm.name = sanitized_enum_name.to_owned();
    //         // enm.database_name = enum_db_name.to_owned();
    //     }

    //     for enum_value in enm.values_mut() {
    //         if &enum_value.name == EMPTY_STRING {
    //             enum_value.name = EMPTY_ENUM_PLACEHOLDER.to_string();
    //             enum_value.database_name = Some(EMPTY_STRING.to_string());
    //         } else {
    //             sanitize_name(&mut enum_value);
    //             // let (sanitized_name, db_name) = sanitize_name(enum_value.name.clone());
    //             // enum_value.name = sanitized_name;
    //             // enum_value.database_name = db_name;
    //         }
    //     }
    // }
}

fn sanitize_names<T>(renameables: &[&mut T])
where
    T: WithDatabaseName + WithName,
{
    // names
    //     .iter_mut()
    //     .map(|f| *f = sanitize_name(f.to_string()).0)
    //     .for_each(drop);
    todo!()
}


fn sanitize_strings(strings: &[String]) -> Vec<String>
{
    // names
    //     .iter_mut()
    //     .map(|f| *f = sanitize_name(f.to_string()).0)
    //     .for_each(drop);
    todo!()
}

// Todo: This is now widely used, we can make this smarter at some point.
// Ideas:
// - Numbers only -> spell out first digit? 100 -> one00
// - Only invalid characters?
// - Underscore at start
fn sanitize_name<T>(renameable: &mut T)
where
    T: WithDatabaseName + WithName,
{
    let name = renameable.name();
    let db_name = renameable.database_name();
    let sanitized = sanitize_string(name.as_str());

    if &sanitized != name {
        // Only set the db name if there's none already set (or else this would invalidate the model).
        if let None = db_name {
            renameable.set_database_name(Some(name.to_owned()));
        }

        renameable.set_name(&sanitized);
    };
}

fn sanitize_string(s: &str) -> String {
    let needs_sanitation = RE_START.is_match(s) || RE.is_match(s);

    if needs_sanitation {
        let start_cleaned: String = RE_START.replace_all(s, "").parse().unwrap();
        let sanitized: String = RE.replace_all(start_cleaned.as_str(), "_").parse().unwrap();

        sanitized
    } else {
        s.to_owned()
    }
}

fn rename_denied(model: &mut Model) {
    if reserved_model_names::is_reserved(model.name()) {
        let name = format!("Renamed{}", model.name);
        let comment = format!(
            "This model has been renamed to '{}' during introspection, because the original name '{}' is reserved.",
            name, model.name,
        );

        match model.documentation {
            Some(ref docs) => model.documentation = Some(format!("{}\n{}", docs, comment)),
            None => model.documentation = Some(comment.to_owned()),
        }

        // Only set @@map if there's no @@map already set.
        if let None = model.database_name {
            model.database_name = Some(model.name.clone());
        }

        model.name = name;
    }
}
