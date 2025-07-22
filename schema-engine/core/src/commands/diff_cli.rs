use std::sync::Arc;

use crate::{
    SchemaContainerExt,
    core_error::CoreResult,
    json_rpc::types::{DiffParams, DiffResult, DiffTarget, UrlContainer},
};
use enumflags2::BitFlags;
use schema_connector::{
    ConnectorError, ConnectorHost, DatabaseSchema, ExternalShadowDatabase, Namespaces, SchemaConnector, SchemaDialect,
    SchemaFilter, migrations_directory::Migrations,
};
use sql_schema_connector::SqlSchemaConnector;

pub async fn diff_cli(params: DiffParams, host: Arc<dyn ConnectorHost>) -> CoreResult<DiffResult> {
    // In order to properly handle MultiSchema, we need to make sure the preview feature is
    // correctly set, and we need to grab the namespaces from the Schema, if any.
    // Note that currently, we union all namespaces and preview features. This may not be correct.
    // TODO: This effectively reads and parses (parts of) the schema twice: once here, and once
    // below, when defining 'from'/'to'. We should revisit this.
    let (namespaces, preview_features) =
        namespaces_and_preview_features_from_diff_targets(&[&params.from, &params.to])?;

    let filter: SchemaFilter = params.filters.into();
    filter.validate(namespaces.as_ref())?;

    let from = json_rpc_diff_target_to_dialect(
        &params.from,
        params.shadow_database_url.as_deref(),
        namespaces.clone(),
        &filter,
        preview_features,
    )
    .await?;
    let to = json_rpc_diff_target_to_dialect(
        &params.to,
        params.shadow_database_url.as_deref(),
        namespaces,
        &filter,
        preview_features,
    )
    .await?;

    // The `from` connector takes precedence, because if we think of diffs as migrations, `from` is
    // the target where the migration would be applied.
    //
    // TODO: make sure the shadow_database_url param is _always_ taken into account.
    // TODO: make sure the connectors are the same in from and to.
    let (dialect, from, to) = match (from, to) {
        (Some((connector, from)), Some((_, to))) => (connector, from, to),
        (Some((connector, from)), None) => {
            let to = connector.empty_database_schema();
            (connector, from, to)
        }
        (None, Some((connector, to))) => {
            let from = connector.empty_database_schema();
            (connector, from, to)
        }
        (None, None) => {
            return Err(ConnectorError::from_msg(
                "Could not determine the connector to use for diffing.".to_owned(),
            ));
        }
    };

    let migration = dialect.diff(from, to, &filter);

    let mut stdout = if params.script {
        dialect.render_script(&migration, &Default::default())?
    } else {
        dialect.migration_summary(&migration)
    };

    if !stdout.ends_with('\n') {
        stdout.push('\n');
    }

    host.print(&stdout).await?;

    let exit_code = if params.exit_code == Some(true) && !dialect.migration_is_empty(&migration) {
        2
    } else {
        0
    };

    Ok(DiffResult {
        exit_code,
        stdout: None,
    })
}

// Grab the preview features and namespaces. Normally, we can only grab these from Schema files,
// and we usually only expect one of these within a set of DiffTarget.
// However, in case there's multiple, we union the results. This may be wrong.
fn namespaces_and_preview_features_from_diff_targets(
    targets: &[&DiffTarget],
) -> CoreResult<(Option<Namespaces>, BitFlags<psl::PreviewFeature>)> {
    let mut namespaces = Vec::new();
    let mut preview_features = BitFlags::default();

    for target in targets {
        match target {
            DiffTarget::Migrations(_) | DiffTarget::Empty | DiffTarget::Url(_) => (),
            DiffTarget::SchemaDatasource(schemas) => {
                let sources = (&schemas.files).to_psl_input();

                ::commands::extract_namespaces(&sources, &mut namespaces, &mut preview_features);
            }
            DiffTarget::SchemaDatamodel(schemas) => {
                let sources = (&schemas.files).to_psl_input();

                ::commands::extract_namespaces(&sources, &mut namespaces, &mut preview_features);
            }
        }
    }

    Ok((Namespaces::from_vec(&mut namespaces), preview_features))
}

// `None` in case the target is empty
async fn json_rpc_diff_target_to_dialect(
    target: &DiffTarget,
    shadow_database_url: Option<&str>, // TODO: delete the parameter
    namespaces: Option<Namespaces>,
    filter: &SchemaFilter,
    preview_features: BitFlags<psl::PreviewFeature>,
) -> CoreResult<Option<(Box<dyn SchemaDialect>, DatabaseSchema)>> {
    match target {
        DiffTarget::Empty => Ok(None),
        DiffTarget::SchemaDatasource(schemas) => {
            let config_dir = std::path::Path::new(&schemas.config_dir);
            let sources: Vec<_> = schemas.to_psl_input();

            // actually, just use the given `connector`. Verify that the provider is the same
            // as the one assumed by the connector.

            let mut connector = crate::schema_to_connector(&sources, Some(config_dir))?;
            connector.ensure_connection_validity().await?;
            connector.set_preview_features(preview_features);
            let schema = connector.schema_from_database(namespaces).await?;
            Ok(Some((connector.schema_dialect(), schema)))
        }
        DiffTarget::SchemaDatamodel(schemas) => {
            let sources = schemas.to_psl_input();
            let dialect = crate::schema_to_dialect(&sources)?;
            let schema = dialect.schema_from_datamodel(sources)?;
            Ok(Some((dialect, schema)))
        }
        DiffTarget::Url(UrlContainer { url }) => {
            // this will not be supported

            let mut connector = crate::connector_for_connection_string(url.clone(), None, BitFlags::empty())?;
            connector.ensure_connection_validity().await?;
            connector.set_preview_features(preview_features);

            let schema = connector.schema_from_database(namespaces).await?;
            let dialect = connector.schema_dialect();

            connector.dispose().await?;

            Ok(Some((dialect, schema)))
        }
        DiffTarget::Migrations(migration_list) => {
            let provider =
                schema_connector::migrations_directory::read_provider_from_lock_file(&migration_list.lockfile);
            match (provider.as_deref(), shadow_database_url) {
                (Some(provider), Some(shadow_database_url)) => {
                    let dialect = ::commands::dialect_for_provider(provider)?;
                    let migrations = Migrations::from_migration_list(migration_list);

                    let schema = dialect
                        .schema_from_migrations_with_target(
                            &migrations,
                            namespaces,
                            filter,
                            ExternalShadowDatabase::ConnectionString {
                                connection_string: shadow_database_url.to_owned(),
                                preview_features,
                            },
                        )
                        .await?;
                    Ok(Some((dialect, schema)))
                }
                (Some("sqlite"), None) => {
                    // TODO: we don't need this branch
                    let mut connector = SqlSchemaConnector::new_sqlite_inmem(preview_features)?;
                    let migrations = Migrations::from_migration_list(migration_list);
                    let schema = connector
                        .schema_from_migrations(&migrations, namespaces, filter)
                        .await?;
                    Ok(Some((connector.schema_dialect(), schema)))
                }
                (Some(_), None) => Err(ConnectorError::from_msg(
                    "You must pass the --shadow-database-url if you want to diff a migrations directory.".to_owned(),
                )),
                (None, _) => Err(ConnectorError::from_msg(
                    "Could not determine the connector from the migrations directory (missing migration_lock.toml)."
                        .to_owned(),
                )),
            }
        }
    }
}
