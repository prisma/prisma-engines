use crate::SqlFamilyTrait;
use datamodel::{
    reserved_model_names::is_reserved_type_name, Datamodel, DefaultKind, DefaultValue, Field, FieldType, IndexField,
    Model, PrimaryKeyField, ValueGenerator, WithDatabaseName, WithName,
};
use introspection_connector::IntrospectionContext;
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use quaint::prelude::SqlFamily;
use regex::Regex;
use std::collections::HashMap;

static EMPTY_ENUM_PLACEHOLDER: &str = "EMPTY_ENUM_VALUE";

static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new("^[^a-zA-Z]+").unwrap());
static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

pub fn sanitize_datamodel_names(datamodel: &mut Datamodel, ctx: &IntrospectionContext) {
    let enum_renames = sanitize_models(datamodel, ctx);
    sanitize_enums(datamodel, &enum_renames);
}

// if after opionated renames we have duplicated names, e.g. a database with
// tables `a` and `_a`, the tables in the data model (`a` and `a`) would
// cause really weird errors
pub fn sanitization_leads_to_duplicate_names(datamodel: &Datamodel) -> bool {
    for model in datamodel.models() {
        let sanitized = sanitize_string(&model.name);

        let dups = datamodel
            .models()
            .filter(|m| sanitize_string(m.name()) == sanitized)
            .count();

        if dups > 1 {
            return true;
        }
    }

    false
}

// Todo: Sanitizing might need to be adjusted to also change the fields in the RelationInfo
fn sanitize_models(datamodel: &mut Datamodel, ctx: &IntrospectionContext) -> HashMap<String, (String, Option<String>)> {
    let mut enum_renames = HashMap::new();

    for model in datamodel.models_mut() {
        rename_reserved(model);
        sanitize_name(model);

        let model_name = model.name().to_owned();
        let model_db_name = model.database_name().map(|s| s.to_owned());

        if let Some(pk) = &mut model.primary_key {
            sanitize_pk_field_names(&mut pk.fields);
        }

        for field in model.fields_mut() {
            sanitize_name(field);

            match field {
                Field::RelationField(rf) => {
                    let info = &mut rf.relation_info;

                    info.name = sanitize_string(&info.name);
                    info.to = sanitize_string(&reformat_reserved_string(&info.to));

                    info.references = sanitize_strings(&info.references);
                    info.fields = sanitize_strings(&info.fields);
                }

                Field::ScalarField(sf) => {
                    if let FieldType::Enum(enum_name) = &sf.field_type {
                        // Enums in MySQL are defined on the column and do not have a separate name.
                        // Introspection generates an enum name for MySQL as `<model_name>_<field_type>`.
                        // If the sanitization changes the enum name, we need to make sure it's changed everywhere.
                        let (sanitized_enum_name, db_name) = if let SqlFamily::Mysql = ctx.sql_family() {
                            if model_db_name.is_none() && sf.database_name.is_none() {
                                (enum_name.to_owned(), None)
                            } else {
                                (format!("{}_{}", model_name, sf.name()), Some(enum_name.to_owned()))
                            }
                        } else {
                            let sanitized = sanitize_string(enum_name);

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
                        if let Some(DefaultKind::Single(PrismaValue::Enum(value))) =
                            sf.default_value.as_mut().map(|dv| dv.mut_kind())
                        {
                            let new_default = if value.is_empty() {
                                DefaultValue::new_single(PrismaValue::Enum(EMPTY_ENUM_PLACEHOLDER.to_string()))
                            } else {
                                let sanitized_value = sanitize_string(value);

                                match sanitized_value {
                                    x if x.is_empty() => {
                                        DefaultValue::new_expression(ValueGenerator::new_dbgenerated(value.clone()))
                                    }
                                    _ => DefaultValue::new_single(PrismaValue::Enum(sanitized_value)),
                                }
                            };

                            sf.default_value.replace(new_default);
                        };
                    }
                }
                Field::CompositeField(_) => todo!(),
            }
        }

        for index in &mut model.indices {
            sanitize_index_field_names(&mut index.fields);
        }
    }

    enum_renames
}

fn sanitize_enums(datamodel: &mut Datamodel, enum_renames: &HashMap<String, (String, Option<String>)>) {
    for enm in datamodel.enums_mut() {
        if let Some((sanitized_name, db_name)) = enum_renames.get(&enm.name) {
            if enm.database_name().is_none() {
                enm.set_database_name(db_name.clone());
            }

            enm.set_name(sanitized_name);
        } else {
            sanitize_name(enm);
        }

        for enum_value in enm.values_mut() {
            if enum_value.name.is_empty() {
                enum_value.name = EMPTY_ENUM_PLACEHOLDER.to_string();
                enum_value.database_name = Some("".to_string());
            } else {
                sanitize_name(enum_value);
            }
        }
    }
}

fn sanitize_pk_field_names(fields: &mut [PrimaryKeyField]) {
    fields
        .iter_mut()
        .map(|mut field| field.name = sanitize_string(&field.name))
        .collect()
}

fn sanitize_index_field_names(fields: &mut [IndexField]) {
    fields
        .iter_mut()
        .map(|mut field| field.name = sanitize_string(&field.name))
        .collect()
}

fn sanitize_strings(strings: &[String]) -> Vec<String> {
    strings.iter().map(|f| sanitize_string(f)).collect()
}

/// We agreed on a simple sanitization logic. Any remaining conflicts will produce a datamodel with
/// name conflicts. Our validation will catch that and ask the user to disambiguate manually.
fn sanitize_name<T>(renameable: &mut T)
where
    T: WithDatabaseName + WithName,
{
    let name = renameable.name().clone();
    let db_name = renameable.database_name().map(|s| s.to_owned());
    let sanitized = sanitize_string(name.as_str());

    if sanitized != name {
        // Only set the db name if there's none already set (or else this would invalidate the model).
        if db_name.is_none() {
            renameable.set_database_name(Some(name));
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
            None => model.documentation = Some(comment),
        }

        // Only set @@map if there's no @@map already set.
        if model.database_name.is_none() {
            model.database_name = Some(model.name.clone());
        }

        model.name = name;
    }
}

/// Reformats a reserved string as "Renamed{}"
fn reformat_reserved_string(s: &str) -> String {
    if is_reserved_type_name(s) {
        format!("Renamed{}", s)
    } else {
        s.to_owned()
    }
}
