use crate::{
    core_error::CoreResult,
    json_rpc::types::{DiffParams, DiffResult, DiffTarget, PathContainer, UrlContainer},
    SchemaContainerExt,
};
use enumflags2::BitFlags;
use schema_connector::{
    ConnectorError, ConnectorHost, DatabaseSchema, DiffTarget as McDiff, Namespaces, SchemaConnector,
};
use std::{path::Path, sync::Arc};

pub async fn diff(params: DiffParams, host: Arc<dyn ConnectorHost>) -> CoreResult<DiffResult> {
    // In order to properly handle MultiSchema, we need to make sure the preview feature is
    // correctly set, and we need to grab the namespaces from the Schema, if any.
    // Note that currently, we union all namespaces and preview features. This may not be correct.
    // TODO: This effectively reads and parses (parts of) the schema twice: once here, and once
    // below, when defining 'from'/'to'. We should revisit this.
    let (namespaces, preview_features) =
        namespaces_and_preview_features_from_diff_targets(&[&params.from, &params.to])?;

    let mut from = json_rpc_diff_target_to_connector(
        &params.from,
        params.shadow_database_url.as_deref(),
        namespaces.clone(),
        preview_features,
    )
    .await?;
    let mut to = json_rpc_diff_target_to_connector(
        &params.to,
        params.shadow_database_url.as_deref(),
        namespaces,
        preview_features,
    )
    .await?;

    for (connector, _) in [from.as_mut(), to.as_mut()].into_iter().flatten() {
        connector.set_host(host.clone());
    }

    // The `from` connector takes precedence, because if we think of diffs as migrations, `from` is
    // the target where the migration would be applied.
    //
    // TODO: make sure the shadow_database_url param is _always_ taken into account.
    // TODO: make sure the connectors are the same in from and to.
    let (connector, from, to) = match (from, to) {
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
            ))
        }
    };

    let migration = connector.diff(from, to);

    if params.script {
        let mut script_string = connector.render_script(&migration, &Default::default())?;
        if !script_string.ends_with('\n') {
            script_string.push('\n');
        }
        connector.host().print(&script_string).await?;
    } else {
        let mut summary = connector.migration_summary(&migration);
        if !summary.ends_with('\n') {
            summary.push('\n');
        }
        connector.host().print(&summary).await?;
    }

    let exit_code = if params.exit_code == Some(true) && !connector.migration_is_empty(&migration) {
        2
    } else {
        0
    };

    Ok(DiffResult { exit_code })
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

fn extract_namespaces(
    files: &[(String, psl::SourceFile)],
    namespaces: &mut Vec<String>,
    preview_features: &mut BitFlags<psl::PreviewFeature>,
) {
    let validated_schema = psl::validate_multi_file(files);

    for (namespace, _span) in validated_schema
        .configuration
        .datasources
        .iter()
        .flat_map(|ds| ds.namespaces.iter())
    {
        namespaces.push(namespace.clone());
    }

    for generator in &validated_schema.configuration.generators {
        *preview_features |= generator.preview_features.unwrap_or_default();
    }
}

// `None` in case the target is empty
async fn json_rpc_diff_target_to_connector(
    target: &DiffTarget,
    shadow_database_url: Option<&str>,
    namespaces: Option<Namespaces>,
    preview_features: BitFlags<psl::PreviewFeature>,
) -> CoreResult<Option<(Box<dyn SchemaConnector>, DatabaseSchema)>> {
    match target {
        DiffTarget::Empty => Ok(None),
        DiffTarget::SchemaDatasource(schemas) => {
            let config_dir = std::path::Path::new(&schemas.config_dir);
            let sources: Vec<_> = schemas.to_psl_input();
            let mut connector = crate::schema_to_connector(&sources, Some(config_dir))?;
            connector.ensure_connection_validity().await?;
            connector.set_preview_features(preview_features);
            let schema = connector
                .database_schema_from_diff_target(McDiff::Database, None, namespaces)
                .await?;
            Ok(Some((connector, schema)))
        }
        DiffTarget::SchemaDatamodel(schemas) => {
            let sources = schemas.to_psl_input();
            let mut connector = crate::schema_to_connector_unchecked(&sources)?;
            connector.set_preview_features(preview_features);

            let schema = connector
                .database_schema_from_diff_target(McDiff::Datamodel(sources), None, namespaces)
                .await?;
            Ok(Some((connector, schema)))
        }
        DiffTarget::Url(UrlContainer { url }) => {
            let mut connector = crate::connector_for_connection_string(url.clone(), None, BitFlags::empty())?;
            connector.ensure_connection_validity().await?;
            connector.set_preview_features(preview_features);
            let schema = connector
                .database_schema_from_diff_target(McDiff::Database, None, namespaces)
                .await?;
            Ok(Some((connector, schema)))
        }
        DiffTarget::Migrations(PathContainer { path }) => {
            let provider = schema_connector::migrations_directory::read_provider_from_lock_file(path);
            match (provider.as_deref(), shadow_database_url) {
                (Some(provider), Some(shadow_database_url)) => {
                    let mut connector = crate::connector_for_provider(provider)?;
                    connector.set_preview_features(preview_features);
                    let directories = schema_connector::migrations_directory::list_migrations(Path::new(path))?;
                    let schema = connector
                        .database_schema_from_diff_target(
                            McDiff::Migrations(&directories),
                            Some(shadow_database_url.to_owned()),
                            namespaces,
                        )
                        .await?;
                    Ok(Some((connector, schema)))
                }
                (Some("sqlite"), None) => {
                    let mut connector = crate::connector_for_provider("sqlite")?;
                    connector.set_preview_features(preview_features);
                    let directories = schema_connector::migrations_directory::list_migrations(Path::new(path))?;
                    let schema = connector
                        .database_schema_from_diff_target(McDiff::Migrations(&directories), None, namespaces)
                        .await?;
                    Ok(Some((connector, schema)))
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
