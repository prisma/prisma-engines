use psl::{
    builtin_connectors::postgres_datamodel_connector::{PostgresDatasourceProperties, PostgresExtension},
    common::preview_features::PreviewFeature,
    Configuration, Datasource, DatasourceConnectorData,
};
use sql_schema_describer::{postgres::PostgresSchemaExt, SqlSchema};

use crate::sanitize_datamodel_names::sanitize_string;

const EXTENSION_ALLOW_LIST: &[&str] = &["citext", "postgis", "pg_crypto", "uuid-ossp"];

pub(super) fn calculate_configuration(previous_config: &Configuration, schema: &SqlSchema) -> Option<Configuration> {
    if !previous_config
        .preview_features()
        .contains(PreviewFeature::PostgresExtensions)
    {
        return None;
    }

    let pg_schema_ext: &PostgresSchemaExt = schema.downcast_connector_data();
    let previous_datasource = previous_config.datasources.first().unwrap();

    let previous_connector_data = previous_datasource
        .connector_data
        .downcast_ref::<PostgresDatasourceProperties>();

    let previous = previous_connector_data
        .and_then(|p| p.extensions())
        .cloned()
        .unwrap_or_default();

    let mut extensions = Vec::new();

    for ext in pg_schema_ext.extension_walkers() {
        let sanitized_name = sanitize_string(ext.name());
        let mut next = PostgresExtension::new(sanitized_name);

        if next.name() != ext.name() {
            next.set_db_name(ext.name().to_owned())
        }

        match previous.find_by_name(ext.name()) {
            Some(prev) => {
                match prev.version() {
                    Some(version) if version != ext.version() => {
                        next.set_version(ext.version().to_owned());
                    }
                    Some(version) => {
                        next.set_version(version.to_owned());
                    }
                    None => (),
                };

                match prev.schema() {
                    Some(schema) if schema != ext.schema() => {
                        next.set_schema(ext.schema().to_owned());
                    }
                    Some(schema) => {
                        next.set_schema(schema.to_owned());
                    }
                    None => (),
                }

                extensions.push(next);
            }
            None if EXTENSION_ALLOW_LIST.contains(&ext.name()) => {
                next.set_schema(ext.schema().to_owned());
                extensions.push(next);
            }
            None => (),
        }
    }

    let mut pg_datasource_ext = PostgresDatasourceProperties::default();
    pg_datasource_ext.set_extensions(extensions);

    let next_datasource = Datasource {
        name: previous_datasource.name.clone(),
        provider: previous_datasource.provider.clone(),
        active_provider: previous_datasource.active_provider,
        url: previous_datasource.url.clone(),
        url_span: previous_datasource.url_span,
        documentation: previous_datasource.documentation.clone(),
        active_connector: previous_datasource.active_connector,
        shadow_database_url: previous_datasource.shadow_database_url.clone(),
        referential_integrity: previous_datasource.referential_integrity,
        relation_mode: previous_datasource.relation_mode,
        schemas: previous_datasource.schemas.clone(),
        schemas_span: previous_datasource.schemas_span,
        connector_data: DatasourceConnectorData::new(Box::new(pg_datasource_ext)),
    };

    Some(Configuration {
        generators: previous_config.generators.clone(),
        datasources: vec![next_datasource],
        warnings: previous_config.warnings.clone(),
    })
}
