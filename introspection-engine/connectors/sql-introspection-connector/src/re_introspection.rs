use datamodel::{Datamodel, FieldType};

pub fn enrich(old_data_model: &Datamodel, new_data_model: &mut Datamodel) {
    //first handle @@map

    //then the other stuff

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
