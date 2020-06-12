use datamodel::{Datamodel, FieldType};

pub fn enrich(old_data_model: &Datamodel, new_data_model: &mut Datamodel) {
    //first handle @@map on models
    let mut changed_model_names = vec![];

    for model in &new_data_model.models {
        if let Some(old_model) = old_data_model
            .models
            .iter()
            .find(|m| m.database_name == Some(model.name.clone()))
        {
            changed_model_names.push((model.name.clone(), old_model.name.clone()))
        }
    }

    for change in changed_model_names {
        let model = new_data_model.find_model_mut(&change.0).unwrap();
        model.name = change.1;
        model.database_name = Some(change.0);
    }

    // @map on fields

    // @@map on enums
    //@map on enum values

    // default values

    //virtual relationfield names

    println!("{:#?}", old_data_model);
    println!("{:#?}", new_data_model);

    let mut virtual_relation_fields_to_be_changed = vec![];

    for model in &new_data_model.models {
        for field in &model.fields {
            if let FieldType::Relation(info) = &field.field_type {
                let old_model = old_data_model.find_model(&model.name).unwrap();

                for old_field in &old_model.fields {
                    if let FieldType::Relation(old_info) = &old_field.field_type {
                        if old_info == info {
                            virtual_relation_fields_to_be_changed.push((
                                model.name.clone(),
                                field.name.clone(),
                                old_field.name.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }

    for change in virtual_relation_fields_to_be_changed {
        let model = new_data_model.find_model_mut(&change.0).unwrap();
        let field = model.find_field_mut(&change.1).unwrap();
        field.name = change.2;
    }

    ()
}
