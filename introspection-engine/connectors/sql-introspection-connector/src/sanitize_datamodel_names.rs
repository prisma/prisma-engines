use datamodel::{DatabaseName::Single, Datamodel, FieldType};
use regex::Regex;

pub fn sanitize_datamodel_names(mut datamodel: Datamodel) -> Datamodel {
    // todo fix name clashes we introduce

    for model in &mut datamodel.models {
        let (sanitized_name, db_name) = sanitize_name(model.name.clone());

        model.name = sanitized_name;
        model.database_name = db_name.map(|db| Single(db));

        for field in &mut model.fields {
            let (sanitized_name, db_name) = sanitize_name(field.name.clone());

            field.name = sanitized_name;

            if field.database_name.is_none() {
                field.database_name = db_name.map(|db| Single(db));
            }

            if let FieldType::Relation(info) = &mut field.field_type {
                info.name = sanitize_name(info.name.clone()).0;
                info.to = sanitize_name(info.to.clone()).0;
                info.to_fields = info.to_fields.iter().map(|f| sanitize_name(f.clone()).0).collect();
            }
        }

        for index in &mut model.indexes {
            index.fields = index.fields.iter().map(|f| sanitize_name(f.clone()).0).collect();
        }
    }

    //    todo enums are a bit more complicated and not fully specced, lets do this separately.
    //    We also need to map the actual valid values before printing them
    //    for enm in &mut datamodel.enums {
    //        let (sanitized_name, db_name) = sanitize_name(enm.name.clone());
    //
    //        enm.name = sanitized_name;
    //        enm.database_name = db_name.map(|db| Single(db));
    //    }

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
