use crate::{
    calculate_datamodel::CalculateDatamodelContext,
    introspection_helpers::{
        compare_options_none_last, replace_index_field_names, replace_pk_field_names, replace_relation_info_field_names,
    },
    warnings::*,
};
use introspection_connector::Warning;
use psl::dml::{self, Datamodel, DefaultValue, Field, Ignorable, ValueGenerator, WithDatabaseName, WithName};
use std::collections::{BTreeSet, HashMap};

pub(crate) fn enrich(old_data_model: &Datamodel, new_data_model: &mut Datamodel, ctx: &mut CalculateDatamodelContext) {
    // Keep @relation attributes even if the database doesn't use foreign keys
    if !ctx.foreign_keys_enabled() {
        merge_relation_fields(old_data_model, new_data_model, ctx.warnings);
    }

    let warnings = &mut ctx.warnings;

    merge_map_attributes_on_models(old_data_model, new_data_model, warnings); //TODO
    merge_pre_3_0_index_names(old_data_model, new_data_model, warnings);
    merge_custom_index_names(old_data_model, new_data_model, warnings);
    merge_changed_primary_key_names(old_data_model, new_data_model, warnings);
    merge_changed_scalar_key_names(old_data_model, new_data_model, warnings);
    merge_changed_relation_field_names(old_data_model, new_data_model);
    merge_changed_relation_names(old_data_model, new_data_model);
    merge_prisma_level_defaults(old_data_model, new_data_model, warnings);
    merge_ignores(old_data_model, new_data_model, warnings);
    merge_comments(old_data_model, new_data_model);
    keep_index_ordering(old_data_model, new_data_model);
}

fn keep_index_ordering(old_data_model: &Datamodel, new_data_model: &mut Datamodel) {
    for old_model in old_data_model.models() {
        let new_model = match new_data_model.models_mut().find(|m| m.name == *old_model.name()) {
            Some(m) => m,
            None => continue,
        };

        new_model.indices.sort_by(|idx_a, idx_b| {
            let idx_a_idx = old_model.indices.iter().position(|idx| idx.db_name == idx_a.db_name);
            let idx_b_idx = old_model.indices.iter().position(|idx| idx.db_name == idx_b.db_name);

            compare_options_none_last(idx_a_idx, idx_b_idx)
        });
    }
}

// Copies `@relation` attributes from the data model to the introspected
// version. Needed, when the database does not support foreign key constraints,
// but we still want to keep them in the PSL.
fn merge_relation_fields(old_data_model: &Datamodel, new_data_model: &mut Datamodel, warnings: &mut Vec<Warning>) {
    let mut changed_models = BTreeSet::new();

    // Maps a model name to the table name it was introspected to. This is helpful when @@map is used.
    // E.g., for
    //
    // ```prisma
    // model Foo {
    //     id     Int @id
    //     bar    Bar @relation(fields: [bar_id], references: [id])
    //     bar_id Int @unique
    //     @@map("foo_table")
    // }
    //
    // the map would be {"Foo" -> "foo_table"}.
    // ```
    let old_model_name_to_final_database_name: HashMap<&str, &str> = old_data_model
        .models()
        .map(|m| (m.name.as_str(), m.final_database_name()))
        .collect();

    for old_model in old_data_model.models() {
        let modifications = new_data_model
            .models()
            .find(|m| *m.final_database_name() == *old_model.final_database_name())
            .map(|new_model| {
                let mut ordering: HashMap<String, usize> = old_model
                    .fields()
                    .enumerate()
                    .map(|(i, field)| (field.name().to_string(), i))
                    .collect();

                for (i, field) in new_model.fields().enumerate() {
                    if !ordering.contains_key(field.name()) {
                        ordering.insert(field.name().to_string(), i);
                    }
                }

                let mut fields = Vec::new();

                for field in old_model.relation_fields() {
                    if new_data_model.models().any(|m| {
                        m.name.as_str()
                            == *old_model_name_to_final_database_name
                                .get(field.relation_info.referenced_model.as_str())
                                .unwrap() // as the old datamodel is guaranteed to be valid at this point, this unwrap is safe
                    }) {
                        fields.push(Field::RelationField(field.clone()));
                    }
                }

                (new_model.name().to_string(), fields, ordering)
            });

        if let Some((model_name, fields, ordering)) = modifications {
            let new_model = new_data_model.find_model_mut(&model_name);

            for field in fields.into_iter() {
                changed_models.insert(new_model.name().to_string());
                new_model.add_field(field);
            }

            new_model
                .fields
                .sort_by_cached_key(|field| *ordering.get(field.name()).unwrap_or(&usize::MAX));
        }
    }

    if !changed_models.is_empty() {
        let affected: Vec<_> = changed_models.into_iter().map(|model| Model { model }).collect();
        warnings.push(warning_relations_added_from_the_previous_data_model(&affected));
    }
}

//@@map on models
fn merge_map_attributes_on_models(
    old_data_model: &Datamodel,
    new_data_model: &mut Datamodel,
    warnings: &mut Vec<Warning>,
) {
    let mut changed_model_names = vec![];

    for model in new_data_model.models() {
        if let Some(old_model) = old_data_model.find_model_db_name(model.database_name.as_ref().unwrap_or(&model.name))
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
            field.relation_info.referenced_model = changed_model_name.1.model.clone();
        }
    }

    if !changed_model_names.is_empty() {
        let models: Vec<_> = changed_model_names.iter().map(|c| c.1.clone()).collect();
        warnings.push(warning_enriched_with_map_on_model(&models));
    }
}

//custom compound index `name` from pre-3.0 datamodels
fn merge_pre_3_0_index_names(old_data_model: &Datamodel, new_data_model: &mut Datamodel, warnings: &mut Vec<Warning>) {
    let mut retained_legacy_index_name_args = vec![];

    for model in new_data_model.models() {
        if let Some(old_model) = &old_data_model.find_model(&model.name) {
            for index in &model.indices {
                if let Some(old_index) = old_model.indices.iter().find(|old| {
                    old.name == index.db_name
                        && old
                            .fields
                            .iter()
                            .map(|f| &f.path.first().unwrap().0)
                            .collect::<Vec<_>>()
                            == index
                                .fields
                                .iter()
                                .map(|f| &f.path.first().unwrap().0)
                                .collect::<Vec<_>>()
                }) {
                    retained_legacy_index_name_args
                        .push(ModelAndIndex::new(&model.name, old_index.name.as_ref().unwrap()))
                }
            }
        }
    }

    //change index name
    for changed_index_name in &retained_legacy_index_name_args {
        let index = new_data_model
            .find_model_mut(&changed_index_name.model)
            .indices
            .iter_mut()
            .find(|i| i.db_name == Some(changed_index_name.index_db_name.to_string()))
            .unwrap();
        index.name = Some(changed_index_name.index_db_name.clone());
    }

    if !retained_legacy_index_name_args.is_empty() {
        let index: Vec<ModelAndIndex> = retained_legacy_index_name_args.to_vec();
        warnings.push(warning_enriched_with_custom_index_names(&index));
    }
}

//custom index names
fn merge_custom_index_names(old_data_model: &Datamodel, new_data_model: &mut Datamodel, warnings: &mut Vec<Warning>) {
    let mut changed_index_names = vec![];

    for model in new_data_model.models() {
        if let Some(old_model) = &old_data_model.find_model(&model.name) {
            for index in &model.indices {
                if let Some(old_index) = old_model.indices.iter().find(|old| old.db_name == index.db_name) {
                    if old_index.name.is_some() {
                        let mf = ModelAndIndex::new(&model.name, old_index.db_name.as_ref().unwrap());
                        changed_index_names.push((mf, old_index.name.clone()))
                    }
                }
            }
        }
    }

    //change index name
    for changed_index_name in &changed_index_names {
        let index = new_data_model
            .find_model_mut(&changed_index_name.0.model)
            .indices
            .iter_mut()
            .find(|i| i.db_name == Some(changed_index_name.0.index_db_name.clone()))
            .unwrap();
        index.name = changed_index_name.1.clone();
    }

    if !changed_index_names.is_empty() {
        let index: Vec<_> = changed_index_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_custom_index_names(&index));
    }
}

//custom primary key names
fn merge_changed_primary_key_names(
    old_data_model: &Datamodel,
    new_data_model: &mut Datamodel,
    warnings: &mut Vec<Warning>,
) {
    let mut changed_primary_key_names = vec![];

    for model in new_data_model.models() {
        if let Some(old_model) = &old_data_model.find_model(&model.name) {
            if let Some(primary_key) = &model.primary_key {
                if let Some(old_primary_key) = &old_model.primary_key {
                    //TODO(extended indices) this should compare more than names at some point
                    if old_primary_key.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
                        == primary_key.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
                        && (old_primary_key.db_name == primary_key.db_name || primary_key.db_name.is_none())
                        && old_primary_key.name.is_some()
                    {
                        let mf = Model::new(&model.name);
                        changed_primary_key_names.push((mf, old_primary_key.name.clone()))
                    }
                }
            }
        }
    }

    //change primary key names
    for changed_primary_key_name in &changed_primary_key_names {
        let pk = new_data_model
            .find_model_mut(&changed_primary_key_name.0.model)
            .primary_key
            .as_mut();

        if let Some(primary_key) = pk {
            primary_key.name = changed_primary_key_name.1.clone()
        }
    }

    if !changed_primary_key_names.is_empty() {
        let pk: Vec<_> = changed_primary_key_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_custom_primary_key_names(&pk));
    }
}

// @map on fields
fn merge_changed_scalar_key_names(
    old_data_model: &Datamodel,
    new_data_model: &mut Datamodel,
    warnings: &mut Vec<Warning>,
) {
    let mut changed_scalar_field_names = vec![];

    for model in new_data_model.models() {
        let old_model = match old_data_model.find_model(&model.name) {
            Some(old_model) => old_model,
            None => continue,
        };

        for field in model.scalar_fields() {
            let old_field =
                match old_model.find_scalar_field_db_name(field.database_name.as_ref().unwrap_or(&field.name)) {
                    Some(old_field) => old_field,
                    None => continue,
                };

            if model.find_scalar_field(&old_field.name).is_none() {
                let mf = ModelAndField::new(&model.name, &field.name);
                changed_scalar_field_names.push((mf, old_field.name.clone()))
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

        if let Some(pk) = &mut model.primary_key {
            replace_pk_field_names(&mut pk.fields, &changed_field_name.0.field, &changed_field_name.1);
        }

        for index in &mut model.indices {
            replace_index_field_names(&mut index.fields, &changed_field_name.0.field, &changed_field_name.1);
        }

        for field in model.relation_fields_mut() {
            replace_relation_info_field_names(
                &mut field.relation_info.fields,
                &changed_field_name.0.field,
                &changed_field_name.1,
            );
        }
    }

    // change RelationInfo.references
    for changed_field_name in &changed_scalar_field_names {
        let fields_to_be_changed = new_data_model.find_relation_fields_for_model(&changed_field_name.0.model);

        for f in fields_to_be_changed {
            let field = new_data_model.find_relation_field_mut(&f.0, &f.1);

            replace_relation_info_field_names(
                &mut field.relation_info.references,
                &changed_field_name.0.field,
                &changed_field_name.1,
            );
        }
    }

    if !changed_scalar_field_names.is_empty() {
        let models_and_fields: Vec<_> = changed_scalar_field_names
            .iter()
            .map(|c| ModelAndField::new(&c.0.model, &c.1))
            .collect();
        warnings.push(warning_enriched_with_map_on_field(&models_and_fields));
    }
}

//always keep old virtual relationfield names
fn merge_changed_relation_field_names(old_data_model: &Datamodel, new_data_model: &mut Datamodel) {
    let mut changed_relation_field_names = vec![];

    for new_model in new_data_model.models() {
        let old_model = match old_data_model.find_model(&new_model.name) {
            Some(old_model) => old_model,
            None => continue,
        };

        for new_field in new_model.relation_fields() {
            for old_field in old_model.relation_fields() {
                let (_, old_related_field) = &old_data_model.find_related_field_bang(old_field);
                let is_many_to_many = old_field.is_list() && old_related_field.is_list();
                let is_self_relation =
                    old_field.relation_info.referenced_model == old_related_field.relation_info.referenced_model;

                let (_, related_field) = &new_data_model.find_related_field_bang(new_field);

                //the relationinfos of both sides need to be compared since the relationinfo of the
                // non-fk side does not contain enough information to uniquely identify the correct relationfield
                let match_as_inline = inline_relation_infos_match(&old_field.relation_info, &new_field.relation_info)
                    && inline_relation_infos_match(&old_related_field.relation_info, &related_field.relation_info);

                let mf = ModelAndField::new(&new_model.name, &new_field.name);

                if match_as_inline
                    || (is_many_to_many
                                //For many to many the relation infos always look the same, here we have to look at the relation name,
                                //which translates to the join table name. But in case of self relations we cannot correctly infer the old name
                                && (old_field.relation_info.name == new_field.relation_info.name && !is_self_relation))
                {
                    changed_relation_field_names.push((mf.clone(), old_field.name.clone()));
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

//keep old virtual relation names on non M:N relations
// M:N relations cannot be uniquely identified without ignoring the relationname and their relationnames cant
// be changed without necessitation db changes since RelationName -> Join table name
fn merge_changed_relation_names(old_data_model: &Datamodel, new_data_model: &mut Datamodel) {
    let mut changed_relation_names = vec![];

    for model in new_data_model.models() {
        let old_model = match old_data_model.find_model(model.name()) {
            Some(old_model) => old_model,
            None => continue,
        };

        for field in model.relation_fields() {
            let (_, related_field) = &new_data_model.find_related_field_bang(field);

            for old_field in old_model.relation_fields() {
                let (_, old_related_field) = &old_data_model.find_related_field_bang(old_field);

                // the relationinfos of both sides need to be compared since the relationinfo of the
                // non-fk side does not contain enough information to uniquely identify the correct relationfield
                let match_as_inline = inline_relation_infos_match(&old_field.relation_info, &field.relation_info)
                    && inline_relation_infos_match(&old_related_field.relation_info, &related_field.relation_info);

                let many_to_many = old_field.is_list() && old_related_field.is_list();

                if match_as_inline && !many_to_many {
                    let mf = ModelAndField::new(&model.name, &field.name);
                    let other_mf = ModelAndField::new(&field.relation_info.referenced_model, &related_field.name);

                    changed_relation_names.push((mf, old_field.relation_info.name.clone()));
                    changed_relation_names.push((other_mf, old_field.relation_info.name.clone()))
                }
            }
        }
    }

    for changed_relation_name in changed_relation_names {
        new_data_model
            .find_relation_field_mut(&changed_relation_name.0.model, &changed_relation_name.0.field)
            .relation_info
            .name = changed_relation_name.1;
    }
}

// Prisma Level Only concepts
// @default(cuid) / @default(uuid) / @updatedAt
fn merge_prisma_level_defaults(
    old_data_model: &Datamodel,
    new_data_model: &mut Datamodel,
    warnings: &mut Vec<Warning>,
) {
    let mut re_introspected_prisma_level_cuids = vec![];
    let mut re_introspected_prisma_level_uuids = vec![];
    let mut re_introspected_updated_at = vec![];

    for model in new_data_model.models() {
        for field in model.scalar_fields() {
            let old_model = match old_data_model.find_model(&model.name) {
                Some(old_model) => old_model,
                None => continue,
            };

            let old_field = match old_model.find_scalar_field(&field.name) {
                Some(mike) => mike, // oldfield
                None => continue,
            };

            if field.default_value.is_none() && field.field_type.is_string() {
                if old_field.default_value == Some(DefaultValue::new_expression(ValueGenerator::new_cuid())) {
                    re_introspected_prisma_level_cuids.push(ModelAndField::new(&model.name, &field.name));
                }

                if old_field.default_value == Some(DefaultValue::new_expression(ValueGenerator::new_uuid())) {
                    re_introspected_prisma_level_uuids.push(ModelAndField::new(&model.name, &field.name));
                }
            }

            if field.field_type.is_datetime() && old_field.is_updated_at {
                re_introspected_updated_at.push(ModelAndField::new(&model.name, &field.name));
            }
        }
    }

    for cuid in &re_introspected_prisma_level_cuids {
        new_data_model
            .find_scalar_field_mut(&cuid.model, &cuid.field)
            .default_value = Some(DefaultValue::new_expression(ValueGenerator::new_cuid()));
    }

    for uuid in &re_introspected_prisma_level_uuids {
        new_data_model
            .find_scalar_field_mut(&uuid.model, &uuid.field)
            .default_value = Some(DefaultValue::new_expression(ValueGenerator::new_uuid()));
    }

    for updated_at in &re_introspected_updated_at {
        new_data_model
            .find_scalar_field_mut(&updated_at.model, &updated_at.field)
            .is_updated_at = true;
    }

    if !re_introspected_prisma_level_cuids.is_empty() {
        warnings.push(warning_enriched_with_cuid(&re_introspected_prisma_level_cuids));
    }

    if !re_introspected_prisma_level_uuids.is_empty() {
        warnings.push(warning_enriched_with_uuid(&re_introspected_prisma_level_uuids));
    }

    if !re_introspected_updated_at.is_empty() {
        warnings.push(warning_enriched_with_updated_at(&re_introspected_updated_at));
    }
}

//@@ignore on models
//@ignore on fields
fn merge_ignores(old_data_model: &Datamodel, new_data_model: &mut Datamodel, warnings: &mut Vec<Warning>) {
    let mut re_introspected_model_ignores = vec![];
    let mut re_introspected_field_ignores = vec![];

    for model in new_data_model.models() {
        let old_model = match old_data_model.find_model(&model.name) {
            Some(old_model) => old_model,
            None => continue,
        };

        if old_model.is_ignored {
            re_introspected_model_ignores.push(Model::new(&model.name));
        }

        for field in model.scalar_fields() {
            let old_field = match old_model.find_scalar_field(&field.name) {
                Some(old_field) => old_field,
                None => continue,
            };

            if old_field.is_ignored {
                re_introspected_field_ignores.push(ModelAndField::new(&model.name, &field.name));
            }
        }
    }

    for ignore in &re_introspected_model_ignores {
        new_data_model.find_model_mut(&ignore.model).is_ignored = true;
    }

    for ignore in &re_introspected_field_ignores {
        new_data_model.find_field_mut(&ignore.model, &ignore.field).ignore();
    }

    if !re_introspected_model_ignores.is_empty() {
        warnings.push(warning_enriched_models_with_ignore(&re_introspected_model_ignores));
    }

    if !re_introspected_field_ignores.is_empty() {
        warnings.push(warning_enriched_fields_with_ignore(&re_introspected_field_ignores));
    }
}

fn merge_comments(old_data_model: &Datamodel, new_data_model: &mut Datamodel) {
    let mut re_introspected_model_comments = vec![];
    let mut re_introspected_field_comments = vec![];

    for model in new_data_model.models() {
        let old_model = match old_data_model.find_model(&model.name) {
            Some(old_model) => old_model,
            None => continue,
        };

        if old_model.documentation.is_some() {
            re_introspected_model_comments.push((Model::new(&model.name), &old_model.documentation))
        }

        for field in &model.fields {
            let old_field = match old_model.find_field(field.name()) {
                Some(old_field) => old_field,
                None => continue,
            };

            if old_field.documentation().is_some() {
                re_introspected_field_comments.push((
                    ModelAndField::new(&model.name, field.name()),
                    old_field.documentation().map(|s| s.to_string()),
                ))
            }
        }
    }

    for model_comment in &re_introspected_model_comments {
        new_data_model.find_model_mut(&model_comment.0.model).documentation = model_comment.1.clone();
    }

    for field_comment in &re_introspected_field_comments {
        new_data_model
            .find_field_mut(&field_comment.0.model, &field_comment.0.field)
            .set_documentation(field_comment.1.clone());
    }

    let mut re_introspected_enum_comments = vec![];
    let mut re_introspected_enum_value_comments = vec![];

    for enm in new_data_model.enums() {
        for value in &enm.values {
            let old_enum = match old_data_model.find_enum(&enm.name) {
                Some(old_enum) => old_enum,
                None => continue,
            };

            if old_enum.documentation.is_some() {
                re_introspected_enum_comments.push((Enum::new(&enm.name), &old_enum.documentation))
            }

            let old_value = match old_enum.find_value(&value.name) {
                Some(old_value) => old_value,
                None => continue,
            };

            if old_value.documentation.is_some() {
                re_introspected_enum_value_comments.push((
                    EnumAndValue::new(&enm.name, &value.name),
                    old_value.documentation.clone(),
                ))
            }
        }
    }

    for enum_comment in &re_introspected_enum_comments {
        new_data_model.find_enum_mut(&enum_comment.0.enm).documentation = enum_comment.1.clone();
    }

    for enum_value_comment in &re_introspected_enum_value_comments {
        new_data_model
            .find_enum_mut(&enum_value_comment.0.enm)
            .find_value_mut(&enum_value_comment.0.value)
            .documentation = enum_value_comment.1.clone();
    }
}

fn inline_relation_infos_match(a: &dml::RelationInfo, b: &dml::RelationInfo) -> bool {
    a.referenced_model == b.referenced_model && a.fields == b.fields && a.references == b.references
}
