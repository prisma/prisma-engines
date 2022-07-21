use crate::{
    core_error::CoreResult,
    json_rpc::types::{DiffParams, DiffResult, DiffTarget, PathContainer, SchemaContainer, UrlContainer},
};
use enumflags2::BitFlags;
use migration_connector::{ConnectorError, ConnectorHost, DatabaseSchema, DiffTarget as McDiff, MigrationConnector};
use psl::parser_database::SourceFile;
use std::{path::Path, sync::Arc};

pub async fn diff(params: DiffParams, host: Arc<dyn ConnectorHost>) -> CoreResult<DiffResult> {
    let mut from = json_rpc_diff_target_to_connector(&params.from, params.shadow_database_url.as_deref()).await?;
    let mut to = json_rpc_diff_target_to_connector(&params.to, params.shadow_database_url.as_deref()).await?;

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

    let migration = connector.diff(from, to)?;

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

// `None` in case the target is empty
async fn json_rpc_diff_target_to_connector(
    target: &DiffTarget,
    shadow_database_url: Option<&str>,
) -> CoreResult<Option<(Box<dyn MigrationConnector>, DatabaseSchema)>> {
    let read_prisma_schema_from_path = |schema_path: &str| -> CoreResult<String> {
        std::fs::read_to_string(schema_path).map_err(|err| {
            ConnectorError::from_source_with_context(
                err,
                format!("Error trying to read Prisma schema file at `{}`.", schema_path).into_boxed_str(),
            )
        })
    };

    match target {
        DiffTarget::Empty => Ok(None),
        DiffTarget::SchemaDatasource(SchemaContainer { schema }) => {
            let schema_contents = read_prisma_schema_from_path(schema)?;
            let schema_dir = std::path::Path::new(schema).parent();
            let mut connector = crate::schema_to_connector(&schema_contents, schema_dir)?;
            connector.ensure_connection_validity().await?;
            let schema = connector
                .database_schema_from_diff_target(McDiff::Database, None)
                .await?;
            Ok(Some((connector, schema)))
        }
        DiffTarget::SchemaDatamodel(SchemaContainer { schema }) => {
            let schema_contents = read_prisma_schema_from_path(schema)?;
            let mut connector = crate::schema_to_connector_unchecked(&schema_contents)?;
            let schema = connector
                .database_schema_from_diff_target(
                    McDiff::Datamodel(SourceFile::new_allocated(Arc::from(schema_contents.into_boxed_str()))),
                    None,
                )
                .await?;
            Ok(Some((connector, schema)))
        }
        DiffTarget::Url(UrlContainer { url }) => {
            let mut connector = crate::connector_for_connection_string(url.clone(), None, BitFlags::empty())?;
            connector.ensure_connection_validity().await?;
            let schema = connector
                .database_schema_from_diff_target(McDiff::Database, None)
                .await?;
            Ok(Some((connector, schema)))
        }
        DiffTarget::Migrations(PathContainer { path }) => {
            let provider = migration_connector::migrations_directory::read_provider_from_lock_file(path);
            match (provider.as_deref(), shadow_database_url) {
                (Some(provider), Some(shadow_database_url)) => {
                    let mut connector = crate::connector_for_provider(provider)?;
                    let directories = migration_connector::migrations_directory::list_migrations(Path::new(path))?;
                    let schema = connector
                        .database_schema_from_diff_target(
                            McDiff::Migrations(&directories),
                            Some(shadow_database_url.to_owned()),
                        )
                        .await?;
                    Ok(Some((connector, schema)))
                }
                (Some("sqlite"), None) => {
                    let mut connector = crate::connector_for_provider("sqlite")?;
                    let directories = migration_connector::migrations_directory::list_migrations(Path::new(path))?;
                    let schema = connector
                        .database_schema_from_diff_target(McDiff::Migrations(&directories), None)
                        .await?;
                    Ok(Some((connector, schema)))
                }
                (Some(_), None) => Err(ConnectorError::from_msg(
                    "You must pass the --shadow-database-url if you want to diff a migrations directory.".to_owned(),
                )),
                (None, _) => Err(ConnectorError::from_msg(
                    "Could not determine the connector from the migrations directory (missing migrations_lock.toml)."
                        .to_owned(),
                )),
            }
        }
    }
}
