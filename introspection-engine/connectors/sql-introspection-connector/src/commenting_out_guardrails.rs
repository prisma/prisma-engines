use datamodel::{Datamodel, FieldArity, FieldType, RelationInfo};

pub fn commenting_out_guardrails(datamodel: &mut Datamodel) {
    let mut commented_model_names = vec![];
    let mut models_with_one_to_one_relation = vec![];

    //todo
    for model in &datamodel.models {
        if model.fields.iter().any(|f| match &f.field_type {
            FieldType::Relation(RelationInfo { to, to_fields, .. }) => {
                let other_model = datamodel.find_model(to).unwrap();
                match to_fields {
                    x if x.len() == 1 => match other_model.find_field(x[0].as_str()).unwrap().arity
                    {
                        FieldArity::Required => true,
                        FieldArity::Optional => true,
                        FieldArity::List => false,
                    },
                    _ => false,
                }
            }
            _ => false,
        }) {
            models_with_one_to_one_relation.push(model.name.clone())
        }
    }

    //models without uniques / ids
    for model in &mut datamodel.models {
        if model.id_fields.is_empty()
            && !model.fields.iter().any(|f| f.is_id || f.is_unique)
            && !model.indices.iter().any(|i| i.is_unique())
            && !models_with_one_to_one_relation.contains(&model.name)
        {
            commented_model_names.push(model.name.clone());
            model.is_commented_out = true;
            model.documentation = Some(
                "The underlying table does not contain a unique identifier and can therefore currently not be handled."
                    .to_string(),
            );
        }
    }

    //fields with an empty name
    for model in &mut datamodel.models {
        for field in &mut model.fields {
            if field.name == "".to_string() {
                field.documentation = Some(
                    "This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*"
                        .to_string(),
                );
                field.name = field.database_names.first().unwrap().to_string();
                field.is_commented_out = true;
            }
        }
    }

    for name in &commented_model_names {
        for model in &mut datamodel.models {
            model.fields.retain(|f| !f.points_to_model(name));
        }
    }
}
