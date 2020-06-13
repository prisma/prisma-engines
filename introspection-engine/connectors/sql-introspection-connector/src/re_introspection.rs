use datamodel::{Datamodel, FieldType};

pub fn enrich(old_data_model: &Datamodel, new_data_model: &mut Datamodel) {
    //todo create a bunch of warnings
    // error handling
    // think about cases where this would not error but could be wrong
    // create warnings for @map
    // do not create warnings for virtual relation field names

    println!("{:#?}", old_data_model);
    println!("{:#?}", new_data_model);

    //@@map on models
    let mut changed_model_names = vec![];
    for model in &new_data_model.models {
        if let Some(old_model) = old_data_model.find_model_db_name(&model.name) {
            if new_data_model.find_model(&old_model.name).is_none() {
                changed_model_names.push((model.name.clone(), old_model.name.clone()))
            }
        }
    }

    for change in changed_model_names {
        let model = new_data_model.find_model_mut(&change.0).unwrap();
        model.name = change.1;
        model.database_name = Some(change.0);
    }

    // @map on fields
    let mut changed_scalar_field_names = vec![];

    for model in &new_data_model.models {
        if let Some(old_model) = &old_data_model.find_model(&model.name) {
            for field in &model.fields {
                if let Some(old_field) = old_model.find_field_db_name(&field.name) {
                    if model.find_field(&old_field.name).is_none() {
                        changed_scalar_field_names
                            .push(((model.name.clone(), field.name.clone()), old_field.name.clone()))
                    }
                }
            }
        }
    }

    for change in changed_scalar_field_names {
        let field = new_data_model.find_field_mut(&change.0).unwrap();
        field.name = (change.0).1;
        field.database_name = Some(change.1);
    }

    //todo
    // @@map on enums
    // @map on enum values

    // @defaults
    //todo
    // potential error: what if there was a db default before and then it got removed, now re-introspection makes it virtual
    // you could not get rid of it

    //virtual relationfield names

    let mut changed_relation_field_names = vec![];

    for model in &new_data_model.models {
        for field in &model.fields {
            if let FieldType::Relation(info) = &field.field_type {
                let old_model = old_data_model.find_model(&model.name).unwrap();

                for old_field in &old_model.fields {
                    if let FieldType::Relation(old_info) = &old_field.field_type {
                        if old_info == info {
                            changed_relation_field_names
                                .push(((model.name.clone(), field.name.clone()), old_field.name.clone()));
                        }
                    }
                }
            }
        }
    }

    for change in changed_relation_field_names {
        let field = new_data_model.find_field_mut(&change.0).unwrap();
        field.name = change.1;
    }

    ()
}
