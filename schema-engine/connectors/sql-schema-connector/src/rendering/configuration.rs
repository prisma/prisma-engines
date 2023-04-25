//! Rendering of the datasource and generator parts of the PSL.

use datamodel_renderer as render;
use psl::Configuration;
use sql_schema_describer::SqlSchema;

/// Render a configuration block.
pub(super) fn render<'a>(
    config: &'a Configuration,
    schema: &'a SqlSchema,
    force_namespaces: Option<&'a [String]>,
) -> render::Configuration<'a> {
    let mut output = render::Configuration::default();
    let prev_ds = config.datasources.first().unwrap();
    let mut datasource = render::configuration::Datasource::from_psl(prev_ds, force_namespaces);

    #[cfg(feature = "postgresql")]
    if prev_ds.active_connector.is_provider("postgres") {
        super::postgres::add_extensions(&mut datasource, schema, config);
    }

    output.push_datasource(datasource);

    for prev in config.generators.iter() {
        output.push_generator(render::configuration::Generator::from_psl(prev));
    }

    output
}
