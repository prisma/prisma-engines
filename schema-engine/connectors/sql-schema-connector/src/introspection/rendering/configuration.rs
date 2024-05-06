//! Rendering of the datasource and generator parts of the PSL.

use datamodel_renderer as render;
use psl::ValidatedSchema;
use sql_schema_describer::SqlSchema;

/// Render a configuration block.
pub(super) fn render<'a>(
    previous_schema: &'a ValidatedSchema,
    schema: &'a SqlSchema,
    force_namespaces: Option<&'a [String]>,
) -> render::Configuration<'a> {
    let (prev_ds_file, prev_ds) = previous_schema.configuration.first_datasource_with_file();
    let prev_ds_file_name = previous_schema.db.file_name(prev_ds_file);

    let mut output = render::Configuration::default();
    let mut datasource = render::configuration::Datasource::from_psl(prev_ds, force_namespaces);

    if prev_ds.active_connector.is_provider("postgres") {
        super::postgres::add_extensions(&mut datasource, schema, &previous_schema.configuration);
    }

    output.push_datasource(prev_ds_file_name.to_owned(), datasource);

    for (prev_gen_file, prev_gen) in previous_schema.configuration.generators_with_files() {
        let prev_gen_file_name = previous_schema.db.file_name(prev_gen_file);

        output.push_generator(
            prev_gen_file_name.to_owned(),
            render::configuration::Generator::from_psl(prev_gen),
        );
    }

    output
}
