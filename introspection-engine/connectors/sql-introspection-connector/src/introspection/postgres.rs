use datamodel_renderer as render;
use psl::{
    builtin_connectors::postgres_datamodel_connector::PostgresDatasourceProperties,
    common::preview_features::PreviewFeature, Configuration,
};
use sql_schema_describer::{postgres::PostgresSchemaExt, SqlSchema};

use crate::sanitize_datamodel_names::sanitize_string;

const EXTENSION_ALLOW_LIST: &[&str] = &["citext", "postgis", "pg_crypto", "uuid-ossp"];

pub(super) fn add_extensions(datasource: &mut render::Datasource<'_>, schema: &SqlSchema, config: &Configuration) {
    if !config.preview_features().contains(PreviewFeature::PostgresExtensions) {
        return;
    }

    let pg_schema_ext: &PostgresSchemaExt = schema.downcast_connector_data();
    let previous_datasource = config.datasources.first().unwrap();

    let connector_data = previous_datasource
        .connector_data
        .downcast_ref::<PostgresDatasourceProperties>();

    let previous_extensions = connector_data.and_then(|p| p.extensions());
    let mut next_extensions = render::Array::default();

    for ext in pg_schema_ext.extension_walkers() {
        let sanitized_name = sanitize_string(ext.name());
        let is_sanitized = ext.name() != sanitized_name;

        let mut next_extension = render::Function::new(sanitized_name);

        if is_sanitized {
            next_extension.push_param(("map", render::Text(ext.name())));
        }

        match previous_extensions.and_then(|e| e.find_by_name(ext.name())) {
            Some(prev) => {
                match prev.version() {
                    Some(previous_version) if previous_version != ext.version() => {
                        next_extension.push_param(("version", render::Text(ext.version())));
                    }
                    Some(previous_version) => {
                        next_extension.push_param(("version", render::Text(previous_version)));
                    }
                    None => (),
                };

                match prev.schema() {
                    Some(previous_schema) if previous_schema != ext.schema() => {
                        next_extension.push_param(("schema", render::Text(ext.schema())));
                    }
                    Some(previous_schema) => {
                        next_extension.push_param(("schema", render::Text(previous_schema)));
                    }
                    None => (),
                }

                next_extensions.push(next_extension);
            }
            None if EXTENSION_ALLOW_LIST.contains(&ext.name()) => {
                next_extension.push_param(("schema", render::Text(ext.schema())));
                next_extensions.push(next_extension);
            }
            None => (),
        }
    }

    if !next_extensions.is_empty() {
        datasource.push_custom_property("extensions", next_extensions);
    }
}
