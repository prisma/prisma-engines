use datamodel::{
    transform::ast_to_dml::reserved_model_names, Datamodel, DefaultValue, Field, FieldType, Model, WithDatabaseName,
    WithName,
};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use quaint::prelude::SqlFamily;
use regex::Regex;
use std::collections::HashMap;

static EMPTY_ENUM_PLACEHOLDER: &'static str = "EMPTY_ENUM_VALUE";
static EMPTY_STRING: &'static str = "";

static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new("^[^a-zA-Z]+").unwrap());
static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

pub fn sanitize_datamodel_names(datamodel: &mut Datamodel, family: &SqlFamily) {
    let enum_renames = sanitize_models(datamodel, family);
    sanitize_enums(datamodel, &enum_renames);
}

// Todo: Sanitizing might need to be adjusted to also change the fields in the RelationInfo
fn sanitize_models(datamodel: &mut Datamodel, family: &SqlFamily) -> HashMap<String, (String, Option<String>)> {
    let mut enum_renames = HashMap::new();

    for model in datamodel.models_mut() {
        rename_reserved(model);
        sanitize_name(model);

        let model_name = model.name().to_owned();
        let model_db_name = model.database_name().map(|s| s.to_owned());

        model.id_fields = sanitize_strings(model.id_fields.as_slice());

        for field in model.fields_mut() {
            sanitize_name(field);

            match field {
                Field::RelationField(rf) => {
                    let info = &mut rf.relation_info;

                    info.name = sanitize_string(&info.name);
                    info.to = sanitize_string(&reformat_reserved_string(&info.to));

                    info.to_fields = sanitize_strings(&info.to_fields);
                    info.fields = sanitize_strings(&info.fields);
                }

                Field::ScalarField(sf) => {
                    if let FieldType::Enum(enum_name) = &sf.field_type {
                        // Enums in MySQL are defined on the column and do not have a separate name.
                        // Introspection generates an enum name for MySQL as `<model_name>_<field_type>`.
                        // If the sanitization changes the enum name, we need to make sure it's changed everywhere.
                        let (sanitized_enum_name, db_name) = if let SqlFamily::Mysql = family {
                            if model_db_name.is_none() && sf.database_name.is_none() {
                                (enum_name.to_owned(), None)
                            } else {
                                (format!("{}_{}", model_name, sf.name()), Some(enum_name.to_owned()))
                            }
                        } else {
                            let sanitized = sanitize_string(&enum_name);

                            if &sanitized != enum_name {
                                (sanitized, Some(enum_name.to_owned()))
                            } else {
                                (sanitized, None)
                            }
                        };

                        if db_name.is_some() {
                            enum_renames.insert(enum_name.to_owned(), (sanitized_enum_name.clone(), db_name));
                        }

                        sf.field_type = FieldType::Enum(sanitized_enum_name);

                        // If the field also has an associated default enum value, we need to sanitize that enum value.
                        // The actual enum value renames _in the enum itself_ are done at a later stage.
                        if let Some(DefaultValue::Single(PrismaValue::Enum(value))) = &mut sf.default_value {
                            let new_default = if EMPTY_STRING == value {
                                DefaultValue::Single(PrismaValue::Enum(EMPTY_ENUM_PLACEHOLDER.to_string()))
                            } else {
                                let sanitized_value = sanitize_string(value);

                                match sanitized_value {
                                    x if x == EMPTY_STRING => DefaultValue::new_db_generated(),
                                    _ => DefaultValue::Single(PrismaValue::Enum(sanitized_value)),
                                }
                            };

                            sf.default_value.replace(new_default);
                        };
                    }
                }
            }
        }

        for index in &mut model.indices {
            index.fields = sanitize_strings(&index.fields);
        }
    }

    enum_renames
}

fn sanitize_enums(datamodel: &mut Datamodel, enum_renames: &HashMap<String, (String, Option<String>)>) {
    for enm in datamodel.enums_mut() {
        if let Some((sanitized_name, db_name)) = enum_renames.get(&enm.name) {
            if let None = enm.database_name() {
                enm.set_database_name(db_name.clone());
            }

            enm.set_name(sanitized_name);
        } else {
            sanitize_name(enm);
        }

        for enum_value in enm.values_mut() {
            if &enum_value.name == EMPTY_STRING {
                enum_value.name = EMPTY_ENUM_PLACEHOLDER.to_string();
                enum_value.database_name = Some(EMPTY_STRING.to_string());
            } else {
                sanitize_name(enum_value);
            }
        }
    }
}

fn sanitize_strings(strings: &[String]) -> Vec<String> {
    strings.into_iter().map(|f| sanitize_string(f)).collect()
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
    let name = renameable.name().clone();
    let db_name = renameable.database_name().map(|s| s.to_owned());
    let sanitized = sanitize_string(name.as_str());

    if sanitized != name {
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

fn rename_reserved(model: &mut Model) {
    let name = reformat_reserved_string(model.name());

    if &name != model.name() {
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

/// Reformats a reserved string as "Renamed{}"
fn reformat_reserved_string(s: &str) -> String {
    let validator = reserved_model_names::ReservedModelNameValidator::new();

    if validator.is_reserved(s) {
        format!("Renamed{}", s)
    } else {
        s.to_owned()
    }
}
