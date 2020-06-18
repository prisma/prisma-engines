use crate::warnings::{
    warning_enriched_with_map_on_enum, warning_enriched_with_map_on_field, warning_enriched_with_map_on_model, Enum,
    Model, ModelAndField,
};
use datamodel::{Datamodel, FieldType};
use introspection_connector::Warning;

pub fn enrich(old_data_model: &Datamodel, new_data_model: &mut Datamodel) -> Vec<Warning> {
    // Notes
    // Relationnames are similar to virtual relationfields, they can be changed arbitrarily
    // investigate dmmf / schema / datamodel / internal datamodel and manual @map changes???

    //todo think about cases where this would not error but could be wrong:
    // what if introspection already had to apply @maps and the user manually changed them?

    //todo What about references to changed names??? @map and @@map
    // models       -> relationfield types, relation names, relationfield names
    // fields       -> relations (to and from fields), indexes, id, unique
    // enums        -> field types
    // enum values  -> default values
    // -
    // Order                                    Status          Tested
    // modelnames                               -> done         yes
    // scalar field names                       -> done         yes
    // scalar index                             -> done         yes
    // scalar unique                            -> done         yes
    // scalar id                                -> done         yes
    // Relationinfo.to                          -> done         yes
    // Relationinfo.fields                      -> done         yes
    // Relationinfo.to_fields
    // Relationinfo.name
    // relation field names                     -> done?
    // enum names                               -> done         yes
    // enum types on scalar fields              -> done         yes
    // enum values
    // enum values in defaults

    println!("{:#?}", old_data_model);
    println!("{:#?}", new_data_model);

    //@@map on models
    let mut changed_model_names = vec![];
    {
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

        //change model names
        for change in &changed_model_names {
            let model = new_data_model.find_model_mut(&change.0.model).unwrap();
            model.name = change.1.clone();
            model.database_name = Some(change.0.model.clone());
        }

        // change relation types
        for change in &changed_model_names {
            let fields_to_be_changed = new_data_model.find_relation_fields_for_model(&change.0.model);

            for relation_field in fields_to_be_changed {
                let field = new_data_model
                    .find_field_mut(&relation_field.0, &relation_field.1)
                    .unwrap();

                if let FieldType::Relation(info) = &mut field.field_type {
                    info.to = change.1.clone();
                }
            }
        }
    }

    // @map on fields
    let mut changed_scalar_field_names = vec![];
    {
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

        //change field name
        for change in &changed_scalar_field_names {
            let field = new_data_model.find_field_mut(&change.0.model, &change.0.field).unwrap();
            field.name = change.1.clone();
            field.database_name = Some(change.0.field.clone());
        }

        // change usages in @@id, @@index, @@unique and on RelationInfo.fields
        for change in &changed_scalar_field_names {
            let model = new_data_model.find_model_mut(&change.0.model).unwrap();

            replace_field_names(&mut model.id_fields, &change.0.field, &change.1);
            for index in &mut model.indices {
                replace_field_names(&mut index.fields, &change.0.field, &change.1);
            }
            for field in &mut model.fields {
                if let FieldType::Relation(info) = &mut field.field_type {
                    replace_field_names(&mut info.fields, &change.0.field, &change.1);
                }
            }
        }

        // change RelationInfo.to_fields
        for change in &changed_scalar_field_names {
            let fields_to_be_changed = new_data_model.find_relation_fields_for_model(&change.0.model);
            for f in fields_to_be_changed {
                let field = new_data_model.find_field_mut(&f.0, &f.1).unwrap();
                if let FieldType::Relation(info) = &mut field.field_type {
                    replace_field_names(&mut info.to_fields, &change.0.field, &change.1);
                }
            }
        }
    }

    // update relation names (needs all fields and models to already be updated)
    {
        for change in &changed_model_names {
            let fields_to_be_changed = new_data_model.find_relation_fields_for_model(&change.0.model);

            // (model, field) -> new Relationname
            for change2 in fields_to_be_changed {
                let model = new_data_model.find_model(&change2.0);
                let field = new_data_model.find_field(&(change2.0, change2.1)).unwrap();

                if let FieldType::Relation(info) = &field.field_type {
                    let other_model = new_data_model.find_model(&info.to).unwrap();
                    //todo I also need the other relationfield and then need to determine which side holds the FK (has non-empty to fields)

                    //         // todo adjust relation name, fieldname just be adjusted later by the virtual fieldnames pass
                    //         // we have logic to suppress the output of the relationname when it matches the expected default relationname
                    //         // otherwise re-introspected models will have their relation names changed
                    //         // if you @map a modelname used in a relation that logic needs to take the original name into account not the current name
                }
            }
        }
    }

    //Then with all this information adjust the relation name
    // let name = if fk_to_same_model.len() < 2 && fk_from_other_model_to_this.is_empty() {
    //     DefaultNames::name_for_unambiguous_relation(model_with_fk, referenced_model)
    // } else {
    //     DefaultNames::name_for_ambiguous_relation(model_with_fk, referenced_model, &fk_column_name)
    // };

    // @@map on enums
    let mut changed_enum_names = vec![];
    {
        for enm in &new_data_model.enums {
            if let Some(old_enum) = old_data_model.find_enum_db_name(&enm.name) {
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
    }

    // todo @map on enum values
    {}

    //virtual relationfield names
    let mut changed_relation_field_names = vec![];
    {
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
    }

    //todo @defaults
    // potential error: what if there was a db default before and then it got removed, now re-introspection makes it virtual
    // you could not get rid of it

    println!("{:#?}", new_data_model);

    //warnings
    let mut warnings = vec![];

    if !changed_model_names.is_empty() {
        let models = changed_model_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_map_on_model(&models));
    }

    if !changed_scalar_field_names.is_empty() {
        let models_and_fields = changed_scalar_field_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_map_on_field(&models_and_fields));
    }

    if !changed_enum_names.is_empty() {
        let enums = changed_enum_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_map_on_enum(&enums));
    }

    warnings
}

fn replace_field_names(target: &mut Vec<String>, old_name: &str, new_name: &str) {
    target
        .iter_mut()
        .map(|v| {
            if v == old_name {
                *v = new_name.to_string()
            }
        })
        .count();
}

// fn replace_enums_in_default_values
