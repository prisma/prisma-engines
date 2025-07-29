//! Rendering of the datasource and generator parts of the PSL.

use datamodel_renderer as render;
use psl::{PreviewFeature, ValidatedSchema};
use sql_schema_describer::{SqlSchema, postgres::PostgresSchemaExt};

/// Render a configuration block.
pub(super) fn render<'a>(
    previous_schema: &'a ValidatedSchema,
    schema: &'a SqlSchema,
    force_namespaces: Option<&'a [String]>,
) -> render::Configuration<'a> {
    let prev_ds = previous_schema.configuration.first_datasource();
    let prev_ds_file_name = previous_schema.db.file_name(prev_ds.span.file_id);

    let mut output = render::Configuration::default();
    let mut datasource = render::configuration::Datasource::from_psl(prev_ds, force_namespaces);

    if prev_ds.active_connector.is_provider("postgres") {
        super::postgres::add_extensions(&mut datasource, schema, &previous_schema.configuration);
    }

    output.push_datasource(prev_ds_file_name.to_owned(), datasource);

    for prev_gen in &previous_schema.configuration.generators {
        let prev_gen_file_name = previous_schema.db.file_name(prev_gen.span.file_id);

        let mut generator = render::configuration::Generator::from_psl(prev_gen);

        // Add partialIndexes preview feature if partial indexes are detected and not already present
        if has_partial_indexes(schema)
            && (prev_ds.active_connector.is_provider("postgresql")
                || prev_ds.active_connector.is_provider("cockroachdb"))
            && !prev_gen
                .preview_features
                .is_some_and(|features| features.contains(PreviewFeature::PartialIndexes))
        {
            generator.push_preview_feature(PreviewFeature::PartialIndexes);
        }

        output.push_generator(prev_gen_file_name.to_owned(), generator);
    }

    output
}

/// Check if the schema contains any partial indexes.
fn has_partial_indexes(schema: &SqlSchema) -> bool {
    let pg_ext: &PostgresSchemaExt = schema.downcast_connector_data();
    !pg_ext.partial_indexes.is_empty()
}
