mod postgres;

use crate::{
    calculate_datamodel::CalculateDatamodelContext as Context,
    commenting_out_guardrails::commenting_out_guardrails,
    introspection_helpers::*,
    prisma_1_defaults::add_prisma_1_id_defaults,
    re_introspection::enrich,
    sanitize_datamodel_names::{sanitization_leads_to_duplicate_names, sanitize_datamodel_names},
    version_checker, SqlError, SqlFamilyTrait,
};
use datamodel_renderer as render;
use introspection_connector::{Version, Warning};
use psl::{
    dml::{self, Datamodel, Field, Model, PrimaryKeyDefinition, PrimaryKeyField, RelationField, SortOrder},
    Configuration,
};
use sql_schema_describer::{walkers::TableWalker, ForeignKeyId, SQLSortOrder, SqlSchema};
use std::collections::HashSet;
use tracing::debug;

pub(crate) fn introspect(ctx: &Context, warnings: &mut Vec<Warning>) -> Result<(Version, String, bool), SqlError> {
    let mut datamodel = Datamodel::new();
    let schema = ctx.schema;

    // collect m2m table names
    let m2m_tables: Vec<String> = schema
        .table_walkers()
        .filter(|table| is_prisma_1_point_1_or_2_join_table(*table) || is_prisma_1_point_0_join_table(*table))
        .map(|table| table.name()[1..].to_string())
        .collect();

    for table in schema
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
            let mut relation_field = calculate_relation_field(foreign_key, &m2m_tables, &duplicated_foreign_keys);

            relation_field.supports_restrict_action(!ctx.sql_family().is_mssql());

            model.add_field(Field::RelationField(relation_field));
        }

        for index in table.indexes() {
            if let Some(index) = calculate_index(index, ctx) {
                model.add_index(index);
            }
        }

        if let Some(pk) = table.primary_key() {
            let clustered = primary_key_is_clustered(pk.id, ctx);

            model.primary_key = Some(PrimaryKeyDefinition {
                name: None,
                db_name: Some(pk.name().to_owned()),
                fields: pk
                    .columns()
                    .map(|c| {
                        let sort_order = c.sort_order().map(|sort| match sort {
                            SQLSortOrder::Asc => SortOrder::Asc,
                            SQLSortOrder::Desc => SortOrder::Desc,
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

        datamodel.add_model(model);
    }

    for e in schema.enum_walkers() {
        let values = e.values().iter().map(|v| dml::EnumValue::new(v)).collect();

        let schema = if matches!(ctx.config.datasources.first(), Some(ds) if !ds.namespaces.is_empty()) {
            e.namespace().map(|n| n.to_string())
        } else {
            None
        };
        datamodel.add_enum(dml::Enum::new(e.name(), values, schema));
    }

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
        enrich(ctx.previous_datamodel, &mut datamodel, ctx, warnings);
        debug!("Enriching datamodel is done.");
    }

    // commenting out models, fields, enums, enum values
    warnings.append(&mut commenting_out_guardrails(&mut datamodel, ctx));

    // try to identify whether the schema was created by a previous Prisma version
    let version = version_checker::check_prisma_version(ctx, warnings);

    // if based on a previous Prisma version add id default opinionations
    add_prisma_1_id_defaults(&version, &mut datamodel, schema, warnings, ctx);

    let config = if ctx.render_config {
        render_configuration(ctx.config, schema).to_string()
    } else {
        String::new()
    };

    let rendered = format!(
        "{}\n{}\n{}",
        config,
        psl::render_datamodel_to_string(&datamodel, Some(ctx.config)),
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
