use crate::{
    calculate_datamodel::CalculateDatamodelContext, introspection_helpers::compare_options_none_last, warnings::*,
};
use introspection_connector::Warning;
use psl::dml::{Datamodel, DefaultValue, ValueGenerator, WithName};

pub(crate) fn enrich(old_data_model: &Datamodel, new_data_model: &mut Datamodel, ctx: &mut CalculateDatamodelContext) {
    let warnings = &mut ctx.warnings;
    merge_pre_3_0_index_names(old_data_model, new_data_model, warnings);
    merge_custom_index_names(old_data_model, new_data_model, warnings);
    merge_prisma_level_defaults(old_data_model, new_data_model, warnings);
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
                    if let Some(ref old_name) = old_index.name {
                        retained_legacy_index_name_args.push(ModelAndIndex::new(&model.name, old_name))
                    }
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
