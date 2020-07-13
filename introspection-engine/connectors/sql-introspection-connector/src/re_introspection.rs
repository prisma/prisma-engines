use crate::warnings::{
    warning_enriched_with_map_on_enum, warning_enriched_with_map_on_enum_value, warning_enriched_with_map_on_field,
    warning_enriched_with_map_on_model, Enum, EnumAndValue, Model, ModelAndField,
};
use datamodel::{Datamodel, DefaultNames, DefaultValue, FieldType};
use introspection_connector::IntrospectionResult;
use prisma_value::PrismaValue;

pub fn enrich(old_data_model: &Datamodel, introspection_result: &mut IntrospectionResult) {
    // Notes
    // Relationnames are similar to virtual relationfields, they can be changed arbitrarily
    // investigate dmmf / schema / datamodel / internal datamodel and manual @map changes???
    // investigate keeping of old manual custom relation names

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
    // Relationinfo.to_fields                   -> done         yes
    // Relationinfo.name                        -> done         yes   -> what if you want to keep that from before?
    // relation field names                     -> done         yes
    // enum names                               -> done         yes
    // enum types on scalar fields              -> done         yes
    // enum values                              -> done         yes
    // enum values in defaults                  -> done         yes

    //todo introspection sometimes has to use @maps itself, which the user can then manually change
    // this has to be handled explicitly -.-also influences the naming in the warnings
    // Order                                    Status          Tested
    // modelnames                               -> done         yes
    // scalar field names                       -> done         yes
    // enum names                               -> done         yes
    // enum values                              -> done         yes

    // println!("{:#?}", old_data_model);
    // println!("{:#?}", introspection_result.datamodel);

    let new_data_model = &mut introspection_result.datamodel;

    //@@map on models
    let mut changed_model_names = vec![];
    {
        for model in &new_data_model.models {
            if let Some(old_model) =
                old_data_model.find_model_db_name(&model.database_name.as_ref().unwrap_or(&model.name))
            {
                if new_data_model.find_model(&old_model.name).is_none() {
                    changed_model_names.push((Model::new(&model.name), Model::new(&old_model.name)))
                }
            }
        }

        //change model names
        for changed_model_name in &changed_model_names {
            let model = new_data_model.find_model_mut(&changed_model_name.0.model);
            model.name = changed_model_name.1.model.clone();
            if model.database_name.is_none() {
                model.database_name = Some(changed_model_name.0.model.clone())
            };
        }

        // change relation types
        for changed_model_name in &changed_model_names {
            let fields_to_be_changed = new_data_model.find_relation_fields_for_model(&changed_model_name.0.model);

            for relation_field in fields_to_be_changed {
                let field = new_data_model.find_relation_field_mut(&relation_field.0, &relation_field.1);
                field.relation_info.to = changed_model_name.1.model.clone();
            }
        }
    }

    // @map on fields
    let mut changed_scalar_field_names = vec![];
    {
        for model in &new_data_model.models {
            if let Some(old_model) = &old_data_model.find_model(&model.name) {
                for field in model.scalar_fields() {
                    if let Some(old_field) =
                        old_model.find_scalar_field_db_name(&field.database_name.as_ref().unwrap_or(&field.name))
                    {
                        if model.find_scalar_field(&old_field.name).is_none() {
                            let mf = ModelAndField::new(&model.name, &field.name);
                            changed_scalar_field_names.push((mf, old_field.name.clone()))
                        }
                    }
                }
            }
        }

        //change field name
        for changed_field_name in &changed_scalar_field_names {
            let field = new_data_model.find_scalar_field_mut(&changed_field_name.0.model, &changed_field_name.0.field);
            field.name = changed_field_name.1.clone();
            if field.database_name.is_none() {
                field.database_name = Some(changed_field_name.0.field.clone())
            };
        }

        // change usages in @@id, @@index, @@unique and on RelationInfo.fields
        for changed_field_name in &changed_scalar_field_names {
            let model = new_data_model.find_model_mut(&changed_field_name.0.model);

            replace_field_names(&mut model.id_fields, &changed_field_name.0.field, &changed_field_name.1);
            for index in &mut model.indices {
                replace_field_names(&mut index.fields, &changed_field_name.0.field, &changed_field_name.1);
            }
            for field in model.relation_fields_mut() {
                replace_field_names(
                    &mut field.relation_info.fields,
                    &changed_field_name.0.field,
                    &changed_field_name.1,
                );
            }
        }

        // change RelationInfo.to_fields
        for changed_field_name in &changed_scalar_field_names {
            let fields_to_be_changed = new_data_model.find_relation_fields_for_model(&changed_field_name.0.model);
            for f in fields_to_be_changed {
                let field = new_data_model.find_relation_field_mut(&f.0, &f.1);
                replace_field_names(
                    &mut field.relation_info.to_fields,
                    &changed_field_name.0.field,
                    &changed_field_name.1,
                );
            }
        }
    }

    // update relation names (needs all fields and models to already be updated)
    // todo this only updates relation names where the name of a model changed.
    // the change of a field name would here not be reflected yet, but these would have been rendered already
    {
        let mut relation_fields_to_change = vec![];
        for changed_model_name in &changed_model_names {
            let changed_model = new_data_model.find_model(&changed_model_name.1.model).unwrap();

            for rf in changed_model.relation_fields() {
                let info = &rf.relation_info;
                let other_model_in_relation = new_data_model.find_model(&info.to).unwrap();
                let number_of_relations_to_other_model_in_relation = changed_model
                    .relation_fields()
                    .filter(|f| f.points_to_model(&info.to))
                    .count();

                let other_relation_field = new_data_model.find_related_field_bang(&rf);
                let other_info = &other_relation_field.relation_info;

                let (model_with_fk, referenced_model, fk_column_name) = if info.to_fields.is_empty() {
                    // does not hold the fk
                    (
                        &other_model_in_relation.name,
                        &changed_model.name,
                        other_info.fields.join("_"),
                    )
                } else {
                    // holds the fk
                    (
                        &changed_model.name,
                        &other_model_in_relation.name,
                        info.fields.join("_"),
                    )
                };

                let unambiguous = number_of_relations_to_other_model_in_relation < 2;
                let relation_name = if unambiguous {
                    DefaultNames::name_for_unambiguous_relation(model_with_fk, referenced_model)
                } else {
                    DefaultNames::name_for_ambiguous_relation(model_with_fk, referenced_model, &fk_column_name)
                };

                relation_fields_to_change.push((changed_model.name.clone(), rf.name.clone(), relation_name.clone()));
                relation_fields_to_change.push((
                    other_model_in_relation.name.clone(),
                    other_relation_field.name.clone(),
                    relation_name.clone(),
                ));
            }
        }

        // change usages in @@id, @@index, @@unique and on RelationInfo.fields
        for changed_relation_field in &relation_fields_to_change {
            let field = new_data_model.find_relation_field_mut(&changed_relation_field.0, &changed_relation_field.1);
            field.relation_info.name = changed_relation_field.2.clone();
        }
    }

    // @@map on enums
    let mut changed_enum_names = vec![];
    {
        for enm in &new_data_model.enums {
            if let Some(old_enum) = old_data_model.find_enum_db_name(&enm.database_name.as_ref().unwrap_or(&enm.name)) {
                if new_data_model.find_enum(&old_enum.name).is_none() {
                    changed_enum_names.push((Enum { enm: enm.name.clone() }, old_enum.name.clone()))
                }
            }
        }
        for changed_enum_name in &changed_enum_names {
            let enm = new_data_model.find_enum_mut(&changed_enum_name.0.enm).unwrap();
            enm.name = changed_enum_name.1.clone();
            if enm.database_name.is_none() {
                enm.database_name = Some(changed_enum_name.0.enm.clone());
            }
        }

        for changed_enum_name in &changed_enum_names {
            let fields_to_be_changed = new_data_model.find_enum_fields(&changed_enum_name.0.enm);

            for change2 in fields_to_be_changed {
                let field = new_data_model.find_scalar_field_mut(&change2.0, &change2.1);
                field.field_type = FieldType::Enum(changed_enum_name.1.clone());
            }
        }
    }

    // @map on enum values
    let mut changed_enum_values = vec![];
    {
        for enm in &new_data_model.enums {
            if let Some(old_enum) = old_data_model.find_enum(&enm.name) {
                for value in &enm.values {
                    if let Some(old_value) =
                        old_enum.find_value_db_name(value.database_name.as_ref().unwrap_or(&value.name.to_owned()))
                    {
                        if enm.find_value(&old_value.name).is_none() {
                            let ev = EnumAndValue::new(&enm.name, &value.name);
                            changed_enum_values.push((ev, old_value.name.clone()))
                        }
                    }
                }
            }
        }
        for changed_enum_value in &changed_enum_values {
            let enm = new_data_model.find_enum_mut(&changed_enum_value.0.enm).unwrap();
            let value = enm.find_value_mut(&changed_enum_value.0.value).unwrap();
            value.name = changed_enum_value.1.clone();
            if value.database_name.is_none() {
                value.database_name = Some(changed_enum_value.0.value.clone());
            }
        }

        for changed_enum_value in &changed_enum_values {
            let fields_to_be_changed = new_data_model.find_enum_fields(&changed_enum_value.0.enm);

            for field in fields_to_be_changed {
                let field = new_data_model.find_scalar_field_mut(&field.0, &field.1);
                if field.default_value
                    == Some(DefaultValue::Single(PrismaValue::Enum(
                        changed_enum_value.0.value.clone(),
                    )))
                {
                    field.default_value = Some(DefaultValue::Single(PrismaValue::Enum(changed_enum_value.1.clone())));
                }
            }
        }
    }

    //virtual relationfield names
    let mut changed_relation_field_names = vec![];
    {
        for model in &new_data_model.models {
            for field in model.relation_fields() {
                if let Some(old_model) = old_data_model.find_model(&model.name) {
                    for old_field in old_model.relation_fields() {
                        //the relationinfos of both sides need to be compared since the relationinfo of the
                        // non-fk side does not contain enough information to uniquely identify the correct relationfield
                        if &old_field.relation_info == &field.relation_info
                            && &old_data_model.find_related_field_bang(&old_field).relation_info
                                == &new_data_model.find_related_field_bang(&field).relation_info
                        {
                            let mf = ModelAndField::new(&model.name, &field.name);
                            changed_relation_field_names.push((mf, old_field.name.clone()));
                        }
                    }
                }
            }
        }

        for changed_relation_field_name in changed_relation_field_names {
            new_data_model
                .find_relation_field_mut(
                    &changed_relation_field_name.0.model,
                    &changed_relation_field_name.0.field,
                )
                .name = changed_relation_field_name.1;
        }
    }

    //todo @defaults
    // potential error: what if there was a db default before and then it got removed, now re-introspection makes it virtual
    // you could not get rid of it

    // println!("{:#?}", new_data_model);

    //warnings
    //todo adjust them to use the new names

    if !changed_model_names.is_empty() {
        let models = changed_model_names.iter().map(|c| c.1.clone()).collect();
        introspection_result
            .warnings
            .push(warning_enriched_with_map_on_model(&models));
    }

    if !changed_scalar_field_names.is_empty() {
        let models_and_fields = changed_scalar_field_names
            .iter()
            .map(|c| ModelAndField::new(&c.0.model, &c.1))
            .collect();
        introspection_result
            .warnings
            .push(warning_enriched_with_map_on_field(&models_and_fields));
    }

    if !changed_enum_names.is_empty() {
        let enums = changed_enum_names.iter().map(|c| Enum::new(&c.1)).collect();
        introspection_result
            .warnings
            .push(warning_enriched_with_map_on_enum(&enums));
    }

    if !changed_enum_values.is_empty() {
        let enums_and_values = changed_enum_values
            .iter()
            .map(|c| EnumAndValue::new(&c.0.enm, &c.1))
            .collect();
        introspection_result
            .warnings
            .push(warning_enriched_with_map_on_enum_value(&enums_and_values));
    }
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
