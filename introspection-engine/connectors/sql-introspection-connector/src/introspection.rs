pub(crate) mod inline_relations;

mod enums;
mod indexes;
mod m2m_relations;
mod models;
mod postgres;
mod prisma_relation_mode;
mod relation_names;

use crate::{calculate_datamodel::CalculateDatamodelContext as Context, SqlError};
use datamodel_renderer as render;
use psl::{dml, Configuration};
use sql_schema_describer::SqlSchema;

pub(crate) fn introspect(ctx: &mut Context) -> Result<(String, bool), SqlError> {
    let mut datamodel = dml::Datamodel::new();

    enums::introspect_enums(ctx);
    models::introspect_models(&mut datamodel, ctx);

    ctx.rendered_schema.push_dml(&ctx.config.datasources[0], &datamodel);

    let relation_names = relation_names::introspect_relation_names(ctx);

    if ctx.foreign_keys_enabled() {
        inline_relations::introspect_inline_relations(&relation_names, ctx);
    } else {
        prisma_relation_mode::reintrospect_relations(ctx);
    }

    m2m_relations::introspect_m2m_relations(&relation_names, ctx);

    let config = if ctx.render_config {
        render_configuration(ctx.config, ctx.schema).to_string()
    } else {
        String::new()
    };

    let rendered = format!("{}\n{}", config, ctx.rendered_schema);

    ctx.finalize_warnings();

    Ok((psl::reformat(&rendered, 2).unwrap(), datamodel.is_empty()))
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
