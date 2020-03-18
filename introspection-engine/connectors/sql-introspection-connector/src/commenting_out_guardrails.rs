use datamodel::{Datamodel, FieldArity, FieldType, RelationInfo};

pub fn commenting_out_guardrails(datamodel: &mut Datamodel) {
    let mut commented_model_names = vec![];
    let mut models_with_one_to_one_relation = vec![];

    for model in &datamodel.models {
        if model
            .fields
            .iter()
            .any(|f| match (&f.arity, &f.field_type) {
                (FieldArity::List, _) => false,
                (
                    _,
                    FieldType::Relation(RelationInfo {
                        to,
                        to_fields: _,
                        name: relation_name,
                        ..
                    }),
                ) => {
                    let other_model = datamodel.find_model(to).unwrap();
                    let other_field = other_model
                        .fields
                        .iter()
                        .find(|f| match &f.field_type {
                            FieldType::Relation(RelationInfo {
                                to: other_to,
                                to_fields: _,
                                name: other_relation_name,
                                ..
                            }) if other_to == &model.name
                                && relation_name == other_relation_name =>
                            {
                                true
                            }
                            _ => false,
                        })
                        .unwrap();

                    match other_field.arity {
                        FieldArity::Optional | FieldArity::Required => true,
                        FieldArity::List => false,
                    }
                }
                (_, _) => false,
            })
        {
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
