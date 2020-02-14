use datamodel::Datamodel;

pub fn comment_out_unhandled_models(datamodel: &mut Datamodel) {
    let mut commented_model_names = vec![];

    for model in &mut datamodel.models {
        if model.id_fields.is_empty()
            && !model.fields.iter().any(|f| f.is_id || f.is_unique)
            && !model.indices.iter().any(|i| i.is_unique())
        {
            commented_model_names.push(model.name.clone());
            model.is_commented_out = true;
            model.documentation = Some(
                "The underlying table does not contain a unique identifier and can therefore currently not be handled."
                    .to_string(),
            );
        }
    }

    for name in &commented_model_names {
        for model in &mut datamodel.models {
            model.fields.retain(|f| !f.points_to_model(name));
        }
    }
}
