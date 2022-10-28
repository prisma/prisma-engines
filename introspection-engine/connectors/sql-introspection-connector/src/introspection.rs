mod postgres;

use crate::{
    calculate_datamodel::CalculateDatamodelContext as Context,
    commenting_out_guardrails::commenting_out_guardrails,
    introspection_helpers::*,
    prisma_1_defaults::add_prisma_1_id_defaults,
    re_introspection::enrich,
    sanitize_datamodel_names::{sanitization_leads_to_duplicate_names, sanitize_datamodel_names},
    version_checker, warnings, SqlError, SqlFamilyTrait,
};
use datamodel_renderer as render;
use introspection_connector::Version;
use psl::{
    datamodel_connector::constraint_names::ConstraintNames,
    dml::{self, Datamodel, Field, Model, PrimaryKeyDefinition, PrimaryKeyField, RelationField, SortOrder},
    parser_database::{ast, walkers},
    Configuration,
};
use sql_schema_describer::{self as sql, walkers::TableWalker, ForeignKeyId, SQLSortOrder, SqlSchema};
use std::collections::{HashMap, HashSet};
use tracing::debug;

pub(crate) fn introspect(ctx: &mut Context) -> Result<(Version, String, bool), SqlError> {
    let mut datamodel = Datamodel::new();
    let schema = ctx.schema;

    introspect_enums(&mut datamodel, ctx);
    introspect_models(&mut datamodel, ctx);

    let mut fields_to_be_added = Vec::new();

    // add backrelation fields
    for model in datamodel.models() {
        for relation_field in model.relation_fields() {
            let relation_info = &relation_field.relation_info;
            if datamodel
                .find_related_field_for_info(relation_info, &relation_field.name)
                .is_none()
            {
                let other_model = datamodel.find_model(&relation_info.referenced_model).unwrap();
                let field = calculate_backrelation_field(schema, model, other_model, relation_field, relation_info)?;

                fields_to_be_added.push((other_model.name.clone(), field));
            }
        }
    }

    // add prisma many to many relation fields
    for table in schema
        .table_walkers()
        .filter(|table| is_prisma_1_point_1_or_2_join_table(*table) || is_prisma_1_point_0_join_table(*table))
    {
        calculate_fields_for_prisma_join_table(table, &mut fields_to_be_added, &datamodel)
    }

    for (model, field) in fields_to_be_added {
        datamodel.find_model_mut(&model).add_field(Field::RelationField(field));
    }

    //TODO(matthias) the sanitation and deduplication of names that come from the schema should move to an initial phase based upon the sqlschema
    //it could yield a map from tableId / enumId to the changed name,
    // during the construction of the dml we'd then draw from that map
    // this way, all usages are already populated with the correct final name and won't need to be tracked down separately
    if !sanitization_leads_to_duplicate_names(&datamodel) {
        // our opinionation about valid names
        sanitize_datamodel_names(ctx, &mut datamodel);
    }

    // TODO(matthias) relation field names might be different since they do not come from the schema but we generate them during dml construction
    // deduplicating relation field names
    deduplicate_relation_field_names(&mut datamodel);

    if !ctx.previous_datamodel.is_empty() {
        enrich(ctx.previous_datamodel, &mut datamodel, ctx);
        debug!("Enriching datamodel is done.");
    }

    // commenting out models, fields, enums, enum values
    ctx.warnings.append(&mut commenting_out_guardrails(&mut datamodel, ctx));

    // try to identify whether the schema was created by a previous Prisma version
    let version = version_checker::check_prisma_version(ctx);

    // if based on a previous Prisma version add id default opinionations
    add_prisma_1_id_defaults(&version, &mut datamodel, schema, ctx);

    let config = if ctx.render_config {
        render_configuration(ctx.config, schema).to_string()
    } else {
        String::new()
    };

    let rendered = format!(
        "{}\n{}",
        config,
        render::Datamodel::from_dml(&ctx.config.datasources[0], &datamodel),
    );

    Ok((version, psl::reformat(&rendered, 2).unwrap(), datamodel.is_empty()))
}

fn render_configuration<'a>(config: &'a Configuration, schema: &'a SqlSchema) -> render::Configuration<'a> {
    let mut output = render::Configuration::default();
    let prev_ds = config.datasources.first().unwrap();
    let mut datasource = render::configuration::Datasource::from_psl(prev_ds);

    if prev_ds.active_connector.is_provider("postgres") {
        postgres::add_extensions(&mut datasource, schema, config);
    }

    output.push_datasource(datasource);

    for prev in config.generators.iter() {
        output.push_generator(render::configuration::Generator::from_psl(prev));
    }

    output
}

fn calculate_fields_for_prisma_join_table(
    join_table: TableWalker<'_>,
    fields_to_be_added: &mut Vec<(String, RelationField)>,
    datamodel: &Datamodel,
) {
    let mut foreign_keys = join_table.foreign_keys();
    if let (Some(fk_a), Some(fk_b)) = (foreign_keys.next(), foreign_keys.next()) {
        let is_self_relation = fk_a.referenced_table().id == fk_b.referenced_table().id;

        for (fk, opposite_fk) in &[(fk_a, fk_b), (fk_b, fk_a)] {
            let referenced_model = dml::find_model_by_db_name(datamodel, fk.referenced_table().name())
                .expect("Could not find model referenced in relation table.");

            let relation_name = join_table.name()[1..].to_owned();
            let field = calculate_many_to_many_field(*opposite_fk, relation_name, is_self_relation);

            fields_to_be_added.push((referenced_model.name.clone(), field));
        }
    }
}

fn introspect_enums(datamodel: &mut Datamodel, ctx: &mut Context<'_>) {
    let mut all_enums: Vec<(Option<ast::EnumId>, dml::Enum)> = ctx
        .schema
        .enum_walkers()
        .map(|enm| {
            let existing_enum = ctx.existing_enum(enm.id);
            let dml_enum = sql_enum_to_dml_enum(enm, existing_enum, ctx);
            (existing_enum.map(|e| e.id), dml_enum)
        })
        .collect();

    all_enums.sort_by(|(id_a, _), (id_b, _)| compare_options_none_last(id_a.as_ref(), id_b.as_ref()));

    if ctx.sql_family().is_mysql() {
        // MySQL can have multiple database enums matching one Prisma enum.
        all_enums.dedup_by(|(id_a, _), (id_b, _)| match (id_a, id_b) {
            (Some(id_a), Some(id_b)) => id_a == id_b,
            _ => false,
        });
    }

    datamodel.enums = all_enums.into_iter().map(|(_id, dml_enum)| dml_enum).collect();
}

fn sql_enum_to_dml_enum(
    sql_enum: sql::EnumWalker<'_>,
    existing_enum: Option<walkers::EnumWalker<'_>>,
    ctx: &mut Context,
) -> dml::Enum {
    let schema = if matches!(ctx.config.datasources.first(), Some(ds) if !ds.namespaces.is_empty()) {
        sql_enum.namespace().map(String::from)
    } else {
        None
    };
    let enum_name = existing_enum.map(|enm| enm.name()).unwrap_or_else(|| sql_enum.name());
    let mut dml_enum = dml::Enum::new(enum_name, Vec::new(), schema);

    dml_enum.database_name = if ctx.sql_family.is_mysql() {
        existing_enum.and_then(|enm| enm.mapped_name()).map(ToOwned::to_owned)
    } else {
        existing_enum
            .filter(|existing_enum| existing_enum.name() != sql_enum.name())
            .map(|_| sql_enum.name().to_owned())
    };

    if dml_enum.database_name.is_some() {
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_enum(&[warnings::Enum::new(
                enum_name,
            )]));
    }

    dml_enum.values.reserve(sql_enum.values().len());

    let mut remapped_values = Vec::new(); // for warnings

    for value in sql_enum.values() {
        let mut dml_value = dml::EnumValue::new(value);

        // Re-introspect mapped names.
        if let Some(existing_value) = existing_enum.and_then(|enm| {
            enm.values()
                .find(|val| val.mapped_name().is_some() && val.database_name() == value)
        }) {
            let mapped_name = std::mem::replace(&mut dml_value.name, existing_value.name().to_owned());
            dml_value.database_name = Some(mapped_name);
            remapped_values.push(warnings::EnumAndValue {
                value: dml_value.name.clone(),
                enm: enum_name.to_owned(),
            });
        }

        dml_enum.values.push(dml_value);
    }

    if !remapped_values.is_empty() {
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_enum_value(&remapped_values))
    }

    dml_enum
}

fn introspect_models(datamodel: &mut Datamodel, ctx: &Context) {
    // collect m2m table names
    let m2m_tables: Vec<String> = ctx
        .schema
        .table_walkers()
        .filter(|table| is_prisma_1_point_1_or_2_join_table(*table) || is_prisma_1_point_0_join_table(*table))
        .map(|table| table.name()[1..].to_string())
        .collect();

    for table in ctx
        .schema
        .table_walkers()
        .filter(|table| !is_old_migration_table(*table))
        .filter(|table| !is_new_migration_table(*table))
        .filter(|table| !is_prisma_1_point_1_or_2_join_table(*table))
        .filter(|table| !is_prisma_1_point_0_join_table(*table))
        .filter(|table| !is_relay_table(*table))
    {
        debug!("Calculating model: {}", table.name());
        let mut model = Model::new(table.name().to_owned(), None);

        for column in table.columns() {
            let field = calculate_scalar_field(column, ctx);
            model.add_field(Field::ScalarField(field));
        }

        let duplicated_foreign_keys: HashSet<ForeignKeyId> = table
            .foreign_keys()
            .enumerate()
            .filter(|(idx, left)| {
                let mut already_visited = table.foreign_keys().take(*idx);
                already_visited.any(|right| {
                    let (left_constrained, right_constrained) =
                        (left.constrained_columns(), right.constrained_columns());
                    left_constrained.len() == right_constrained.len()
                        && left_constrained
                            .zip(right_constrained)
                            .all(|(left, right)| left.id == right.id)
                        && left
                            .referenced_columns()
                            .zip(right.referenced_columns())
                            .all(|(left, right)| left.id == right.id)
                })
            })
            .map(|(_, fk)| fk.id)
            .collect();

        for foreign_key in table
            .foreign_keys()
            .filter(|fk| !duplicated_foreign_keys.contains(&fk.id))
        {
            let relation_field = calculate_relation_field(ctx, foreign_key, &m2m_tables, &duplicated_foreign_keys);
            model.add_field(Field::RelationField(relation_field));
        }

        for index in table.indexes() {
            if let Some(index) = calculate_index(index, ctx) {
                model.add_index(index);
            }
        }

        if let Some(pk) = table.primary_key() {
            let clustered = primary_key_is_clustered(pk.id, ctx);

            let db_name = if pk.name() == ConstraintNames::primary_key_name(table.name(), ctx.active_connector())
                || pk.name().is_empty()
            {
                None
            } else {
                Some(pk.name().to_owned())
            };

            model.primary_key = Some(PrimaryKeyDefinition {
                name: None,
                db_name,
                fields: pk
                    .columns()
                    .map(|c| {
                        let sort_order = c.sort_order().and_then(|sort| match sort {
                            SQLSortOrder::Asc => None,
                            SQLSortOrder::Desc => Some(SortOrder::Desc),
                        });

                        PrimaryKeyField {
                            name: c.name().to_string(),
                            sort_order,
                            length: c.length(),
                        }
                    })
                    .collect(),
                defined_on_field: pk.columns().len() == 1,
                clustered,
            });
        }

        if matches!(ctx.config.datasources.first(), Some(ds) if !ds.namespaces.is_empty()) {
            model.schema = table.namespace().map(|n| n.to_string());
        }

        datamodel.models.push(model);
    }

    sort_models(datamodel, ctx)
}

fn sort_models(datamodel: &mut Datamodel, ctx: &Context) {
    let existing_models_by_database_name: HashMap<&str, _> = ctx
        .previous_schema
        .db
        .walk_models()
        .map(|model| (model.database_name(), model.id))
        .collect();

    datamodel.models.sort_by(|a, b| {
        let existing = |model: &dml::Model| -> Option<_> {
            existing_models_by_database_name.get(model.database_name.as_deref().unwrap_or(&model.name))
        };

        compare_options_none_last(existing(a), existing(b))
    });
}
