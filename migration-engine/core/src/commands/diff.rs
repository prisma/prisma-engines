use crate::{
    core_error::CoreResult,
    json_rpc::types::{DiffParams, DiffResult, DiffTarget, PathContainer, SchemaContainer, UrlContainer},
};
use migration_connector::{ConnectorError, DiffTarget as McDiff};
use std::path::Path;

pub(crate) async fn diff(
    params: &DiffParams,
    ambient_connector: &dyn migration_connector::MigrationConnector,
) -> CoreResult<DiffResult> {
    let mut from =
        json_rpc_diff_target_to_migration_connector_diff_target(&params.from, params.shadow_database_url.as_deref())?;
    let mut to =
        json_rpc_diff_target_to_migration_connector_diff_target(&params.to, params.shadow_database_url.as_deref())?;

    for connector in [from.connector.as_mut(), to.connector.as_mut()].into_iter().flatten() {
        connector.set_host(ambient_connector.host().clone());
    }

    // The `from` connector takes precedence, because if we think of diffs as migrations, `from` is
    // the target where the migration would be applied.
    //
    // TODO: make sure the shadow_database_url param is _always_ taken into account.
    // TODO: make sure the connectors are the same in from and to.
    let connector_from_targets = from
        .connector
        .as_ref()
        .or_else(|| to.connector.as_ref())
        .map(|api| api.connector());

    let connector = connector_from_targets.unwrap_or(ambient_connector); // fall back to the one the ME was initialized with

    let migration: migration_connector::Migration = connector.diff(from.target, to.target).await?;

    if params.script {
        let script_string = connector.database_migration_step_applier().render_script(
            &migration,
            &migration_connector::DestructiveChangeDiagnostics::default(),
        );
        connector.host().print(&script_string).await?;
    } else {
        let summary = connector.migration_summary(&migration);
        connector.host().print(&summary).await?;
    }

    Ok(DiffResult { exit_code: 0 })
}

struct RefinedDiffTarget {
    target: McDiff<'static>,
    /// `None` in case the target is Empty.
    connector: Option<Box<dyn crate::GenericApi>>,
}

// -> CoreResult<(DiffTarget, Option<connector>)> ?
fn json_rpc_diff_target_to_migration_connector_diff_target(
    target: &DiffTarget,
    shadow_database_url: Option<&str>,
) -> CoreResult<RefinedDiffTarget> {
    match target {
        DiffTarget::Empty => Ok(RefinedDiffTarget {
            target: McDiff::Empty,
            connector: None,
        }),
        DiffTarget::SchemaDatasource(SchemaContainer { schema }) => {
            let schema_contents = std::fs::read_to_string(&schema)
                .map_err(|err| ConnectorError::from_source(err, "Reading Prisma schema file."))?;
            let (_, url, _, _) = crate::parse_configuration(&schema_contents)?;
            let api = crate::migration_api(&schema_contents)?;
            Ok(RefinedDiffTarget {
                connector: Some(api),
                target: McDiff::Database(url.into()),
            })
        }
        DiffTarget::SchemaDatamodel(SchemaContainer { schema }) => {
            let schema_contents = std::fs::read_to_string(&schema)
                .map_err(|err| ConnectorError::from_source(err, "Reading Prisma schema file."))?;
            Ok(RefinedDiffTarget {
                connector: Some(crate::migration_api(&schema_contents)?),
                target: McDiff::Datamodel(schema_contents.into()),
            })
        }
        DiffTarget::Url(UrlContainer { url }) => {
            let schema_contents = crate::datasource_from_database_str(url)?;
            let api = crate::migration_api(&schema_contents)?;
            Ok(RefinedDiffTarget {
                connector: Some(api),
                target: McDiff::Database(url.to_owned().into()),
            })
        }
        DiffTarget::Migrations(PathContainer { path }) => {
            let provider = migration_connector::migrations_directory::read_provider_from_lock_file(path);
            let connector = match (provider, shadow_database_url) {
                (Some(provider), Some(_)) => {
                    let maybe_shadow_database_url = shadow_database_url
                        .map(|sdurl| format!("shadowDatabaseUrl = \"{sdurl}\"", sdurl = sdurl.replace('\\', "\\\\")))
                        .unwrap_or_else(String::new);

                    Some(crate::migration_api(&format!(
                        r#"
                            datasource db {{
                                provider = "{provider}"
                                url = "{url}"
                                {maybe_shadow_database_url}
                            }}
                       "#,
                        url = provider_to_dummy_url(&provider)
                    ))?)
                }
                (provider, None) if provider.as_deref() != Some("sqlite") => {
                    return Err(ConnectorError::from_msg(
                        "You must pass the --shadow-database-url if you want to diff a migrations directory."
                            .to_owned(),
                    ))
                }
                (None, _) => return Err(ConnectorError::from_msg(
                    "Could not determine the connector from the migrations directory (missing migrations_lock.toml)."
                        .to_owned(),
                )),
                _ => None,
            };
            let directories = migration_connector::migrations_directory::list_migrations(Path::new(path))?;
            Ok(RefinedDiffTarget {
                connector,
                target: McDiff::Migrations(directories.into()),
            })
        }
    }
}

fn provider_to_dummy_url(provider: &str) -> &'static str {
    match provider {
        "postgresql" | "postgres" | "cockroachdb" => "postgresql://example.com/",
        "mysql" => "mysql://example.com/",
        "sqlserver" => "sqlserver://",
        _ => "<unknown provider>",
    }
}
