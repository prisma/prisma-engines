use crate::{
    CoreError, SchemaContainerExt,
    core_error::CoreResult,
    dialect_for_provider, extract_namespaces,
    json_rpc::types::{DiffParams, DiffResult, DiffTarget, UrlContainer},
};
use enumflags2::BitFlags;
use json_rpc::types::MigrationList;
use psl::SourceFile;
use schema_connector::{
    ConnectorError, DatabaseSchema, ExternalShadowDatabase, Namespaces, SchemaConnector, SchemaDialect,
};

pub async fn diff(params: DiffParams, connector: &mut dyn SchemaConnector) -> CoreResult<DiffResult> {
    // In order to properly handle MultiSchema, we need to make sure the preview feature is
    // correctly set, and we need to grab the namespaces from the Schema, if any.
    // Note that currently, we union all namespaces and preview features. This may not be correct.
    // TODO: This effectively reads and parses (parts of) the schema twice: once here, and once
    // below, when defining 'from'/'to'. We should revisit this.
    let (namespaces, preview_features) =
        namespaces_and_preview_features_from_diff_targets(&[&params.from, &params.to])?;

    let (conn_from, schema_from) = diff_target_to_dialect(
        &params.from,
        params.shadow_database_url.as_deref(),
        connector,
        namespaces.clone(),
        preview_features,
    )
    .await?
    .unzip();

    let (conn_to, schema_to) = diff_target_to_dialect(
        &params.to,
        params.shadow_database_url.as_deref(),
        connector,
        namespaces,
        preview_features,
    )
    .await?
    .unzip();

    let dialect = conn_from
        .or(conn_to)
        .ok_or_else(|| ConnectorError::from_msg("Could not determine the connector to use for diffing.".to_owned()))?;

    // The `from` connector takes precedence, because if we think of diffs as migrations, `from` is
    // the target where the migration would be applied.
    let from = schema_from.unwrap_or_else(|| dialect.empty_database_schema());
    let to = schema_to.unwrap_or_else(|| dialect.empty_database_schema());

    let migration = dialect.diff(from, to);

    let mut stdout = if params.script {
        dialect.render_script(&migration, &Default::default())?
    } else {
        dialect.migration_summary(&migration)
    };

    if !stdout.ends_with('\n') {
        stdout.push('\n');
    }

    let exit_code = if params.exit_code == Some(true) && !dialect.migration_is_empty(&migration) {
        2
    } else {
        0
    };

    Ok(DiffResult {
        exit_code,
        stdout: Some(stdout),
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

                extract_namespaces(&sources, &mut namespaces, &mut preview_features);
            }
            DiffTarget::SchemaDatamodel(schemas) => {
                let sources = (&schemas.files).to_psl_input();

                extract_namespaces(&sources, &mut namespaces, &mut preview_features);
            }
        }
    }

    Ok((Namespaces::from_vec(&mut namespaces), preview_features))
}

async fn diff_target_to_dialect(
    target: &DiffTarget,
    shadow_database_url: Option<&str>,
    connector: &mut dyn SchemaConnector,
    namespaces: Option<Namespaces>,
    preview_features: BitFlags<psl::PreviewFeature>,
) -> CoreResult<Option<(Box<dyn SchemaDialect>, DatabaseSchema)>> {
    match target {
        DiffTarget::Empty => Ok(None),
        DiffTarget::SchemaDatasource(_schemas) => {
            // TODO: verify that the provider is the same as the one assumed by the connector:
            // Note: let's simplify the parsed `provider` value before doing this.
            // ```
            // let config_dir = std::path::Path::new(&schemas.config_dir);
            // let sources: Vec<_> = schemas.to_psl_input();
            // let (_, config) = psl::parse_configuration_multi_file(&sources)?;
            // ```

            connector.ensure_connection_validity().await?;
            connector.set_preview_features(preview_features);
            let schema = connector.schema_from_database(namespaces).await?;
            Ok(Some((connector.schema_dialect(), schema)))
        }
        DiffTarget::SchemaDatamodel(schemas) => {
            let sources = schemas.to_psl_input();
            let dialect = schema_to_dialect(&sources)?;
            let schema = dialect.schema_from_datamodel(sources)?;
            Ok(Some((dialect, schema)))
        }
        DiffTarget::Url(UrlContainer { .. }) => Err(ConnectorError::from_msg(
            "--from-url and --to-url flags are no longer supported".to_owned(),
        )),
        DiffTarget::Migrations(MigrationList {
            lockfile,
            migration_directories,
            ..
        }) => {
            let provider = schema_connector::migrations_directory::read_provider_from_lock_file(lockfile);
            match (provider.as_deref(), shadow_database_url) {
                (Some(provider), Some(shadow_database_url)) => {
                    let dialect = dialect_for_provider(provider)?;
                    let directories =
                        schema_connector::migrations_directory::list_migrations(migration_directories.clone());

                    // TODO: enable Driver Adapter for shadow database, using the AdapterFactory.
                    let schema = dialect
                        .schema_from_migrations_with_target(
                            &directories,
                            namespaces,
                            ExternalShadowDatabase::ConnectionString {
                                connection_string: shadow_database_url.to_owned(),
                                preview_features,
                            },
                        )
                        .await?;
                    Ok(Some((dialect, schema)))
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

fn schema_to_dialect(files: &[(String, SourceFile)]) -> CoreResult<Box<dyn schema_connector::SchemaDialect>> {
    let (_, config) = psl::parse_configuration_multi_file(files)
        .map_err(|(files, err)| CoreError::new_schema_parser_error(files.render_diagnostics(&err)))?;

    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    dialect_for_provider(source.active_provider)
}
