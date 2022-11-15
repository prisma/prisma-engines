pub(crate) mod inline_relations;

mod enums;
mod indexes;
mod m2m_relations;
mod models;
mod postgres;
mod prisma_relation_mode;
mod relation_names;

use crate::{
    calculate_datamodel::CalculateDatamodelContext as Context, commenting_out_guardrails::commenting_out_guardrails,
    prisma_1_defaults::add_prisma_1_id_defaults, re_introspection::enrich, version_checker, SqlError,
};
use datamodel_renderer as render;
use introspection_connector::Version;
use psl::{dml, Configuration};
use sql_schema_describer::SqlSchema;

pub(crate) fn introspect(ctx: &mut Context) -> Result<(Version, String, bool), SqlError> {
    let mut datamodel = dml::Datamodel::new();

    enums::introspect_enums(&mut datamodel, ctx);
    models::introspect_models(&mut datamodel, ctx);

    let relation_names = relation_names::introspect_relation_names(ctx);

    if ctx.foreign_keys_enabled() {
        inline_relations::introspect_inline_relations(&relation_names, &mut datamodel, ctx);
    } else {
        prisma_relation_mode::reintrospect_relations(&mut datamodel, ctx);
    }

    if !ctx.previous_datamodel.is_empty() {
        enrich(ctx.previous_datamodel, &mut datamodel, ctx);
    }

    // commenting out models, fields, enums, enum values
    ctx.warnings.append(&mut commenting_out_guardrails(&mut datamodel, ctx));

    // try to identify whether the schema was created by a previous Prisma version
    let version = version_checker::check_prisma_version(ctx);

    // if based on a previous Prisma version add id default opinionations
    add_prisma_1_id_defaults(&version, &mut datamodel, ctx.schema, ctx);

    m2m_relations::introspect_m2m_relations(&relation_names, &mut datamodel, ctx);

    // Ordering of model fields.
    //
    // This sorts backrelation field after relation fields, in order to preserve an ordering
    // similar to that of the previous implementation.
    for model in &mut datamodel.models {
        model
            .fields
            .sort_by(|a, b| match (a.as_relation_field(), b.as_relation_field()) {
                (Some(a), Some(b)) if a.relation_info.fields.is_empty() && !b.relation_info.fields.is_empty() => {
                    std::cmp::Ordering::Greater // back relation fields last
                }
                (Some(a), Some(b)) if b.relation_info.fields.is_empty() && !a.relation_info.fields.is_empty() => {
                    std::cmp::Ordering::Less
                }
                _ => std::cmp::Ordering::Equal,
            });
    }

    let config = if ctx.render_config {
        render_configuration(ctx.config, ctx.schema).to_string()
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
