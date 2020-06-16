use crate::warnings::{
    warning_enriched_with_map_on_enum, warning_enriched_with_map_on_field, warning_enriched_with_map_on_model, Enum,
    Model, ModelAndField,
};
use datamodel::{Datamodel, FieldType};
use introspection_connector::Warning;

pub fn enrich(old_data_model: &Datamodel, new_data_model: &mut Datamodel) -> Vec<Warning> {
    // Relationnames are similar to virtual relationfields, they can be changed arbitrarily

    //todo create a bunch of warnings
    // error handling
    // think about cases where this would not error but could be wrong
    // create warnings for @map
    // do not create warnings for virtual relation field names

    //todo What about references to changed names??? @map and @@map
    // models       -> relations
    // fields       -> relations, indexes, id, unique
    // enums        -> fields
    // enum values  -> default values

    println!("{:#?}", old_data_model);
    println!("{:#?}", new_data_model);

    let mut warnings = vec![];

    //@@map on models
    let mut changed_model_names = vec![];
    for model in &new_data_model.models {
        if let Some(old_model) = old_data_model.find_model_db_name(&model.name) {
            if new_data_model.find_model(&old_model.name).is_none() {
                changed_model_names.push((
                    Model {
                        model: model.name.clone(),
                    },
                    old_model.name.clone(),
                ))
            }
        }
    }
    for change in &changed_model_names {
        let model = new_data_model.find_model_mut(&change.0.model).unwrap();
        model.name = change.1.clone();
        model.database_name = Some(change.0.model.clone());
    }

    if !changed_model_names.is_empty() {
        let models = changed_model_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_map_on_model(&models));
    }

    // @map on fields
    let mut changed_scalar_field_names = vec![];
    for model in &new_data_model.models {
        if let Some(old_model) = &old_data_model.find_model(&model.name) {
            for field in &model.fields {
                if let Some(old_field) = old_model.find_field_db_name(&field.name) {
                    if model.find_field(&old_field.name).is_none() {
                        changed_scalar_field_names.push((
                            ModelAndField {
                                model: model.name.clone(),
                                field: field.name.clone(),
                            },
                            old_field.name.clone(),
                        ))
                    }
                }
            }
        }
    }

    for change in &changed_scalar_field_names {
        let field = new_data_model.find_field_mut(&change.0.model, &change.0.field).unwrap();
        field.name = change.1.clone();
        field.database_name = Some(change.0.field.clone());
    }

    if !changed_scalar_field_names.is_empty() {
        let models_and_fields = changed_scalar_field_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_map_on_field(&models_and_fields));
    }

    //todo
    // @@map on enums
    let mut changed_enum_names = vec![];
    for enm in &new_data_model.enums {
        if let Some(old_enum) = old_data_model.find_enum_db_name(&enm.name) {
            println!("{:?}", enm);
            println!("{:?}", old_enum);
            if new_data_model.find_enum(&old_enum.name).is_none() {
                changed_enum_names.push((Enum { enm: enm.name.clone() }, old_enum.name.clone()))
            }
        }
    }
    for change in &changed_enum_names {
        let enm = new_data_model.find_enum_mut(&change.0.enm).unwrap();
        enm.name = change.1.clone();
        enm.database_name = Some(change.0.enm.clone());
    }

    for change in &changed_enum_names {
        let fields_to_be_changed = new_data_model.find_enum_fields(&change.0.enm);

        for change2 in fields_to_be_changed {
            let field = new_data_model.find_field_mut(&change2.0, &change2.1).unwrap();
            field.field_type = FieldType::Enum(change.1.clone());
        }
    }

    if !changed_enum_names.is_empty() {
        let enums = changed_enum_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_map_on_enum(&enums));
    }

    //todo
    // @map on enum values

    //virtual relationfield names
    let mut changed_relation_field_names = vec![];
    for model in &new_data_model.models {
        for field in &model.fields {
            if let FieldType::Relation(info) = &field.field_type {
                if let Some(old_model) = old_data_model.find_model(&model.name) {
                    for old_field in &old_model.fields {
                        if let FieldType::Relation(old_info) = &old_field.field_type {
                            if old_info == info {
                                changed_relation_field_names.push((
                                    ModelAndField {
                                        model: model.name.clone(),
                                        field: field.name.clone(),
                                    },
                                    old_field.name.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    for change in changed_relation_field_names {
        let field = new_data_model.find_field_mut(&change.0.model, &change.0.field).unwrap();
        field.name = change.1;
    }

    //todo
    // @defaults
    // potential error: what if there was a db default before and then it got removed, now re-introspection makes it virtual
    // you could not get rid of it

    println!("{:?}", new_data_model);

    warnings
}
