use std::sync::Arc;

use crate::{
    DatasourceUrls, SchemaContainerExt,
    core_error::{CoreError, CoreResult},
    json_rpc::types::{DiffParams, DiffResult, DiffTarget, MigrationList, UrlContainer},
};
use enumflags2::BitFlags;
use psl::{builtin_connectors::BUILTIN_CONNECTORS, datamodel_connector::Flavour, parser_database::ExtensionTypes};
use schema_connector::{
    ConnectorError, ConnectorHost, DatabaseSchema, ExternalShadowDatabase, Namespaces, SchemaConnector, SchemaDialect,
    SchemaFilter, migrations_directory::Migrations,
};
use sql_schema_connector::SqlSchemaConnector;
use user_facing_errors::schema_engine::ShadowDbSameAsMainDb;

pub async fn diff_cli(
    params: DiffParams,
    datasource_urls: &DatasourceUrls,
    host: Arc<dyn ConnectorHost>,
    initial_preview_features: BitFlags<psl::PreviewFeature>,
    extension_types: &dyn ExtensionTypes,
) -> CoreResult<DiffResult> {
    validate_shadow_database_is_not_diffed(&params, datasource_urls)?;

    // In order to properly handle MultiSchema, we need to make sure the preview feature is
    // correctly set, and we need to grab the namespaces from the Schema, if any.
    // Note that currently, we union all namespaces and preview features. This may not be correct.
    // TODO: This effectively reads and parses (parts of) the schema twice: once here, and once
    // below, when defining 'from'/'to'. We should revisit this.
    let (namespaces, preview_features) =
        namespaces_and_preview_features_from_diff_targets(&[&params.from, &params.to])?;
    let preview_features = preview_features | initial_preview_features;

    let filter: SchemaFilter = params.filters.into();

    let from = json_rpc_diff_target_to_dialect(
        &params.from,
        datasource_urls,
        namespaces.clone(),
        &filter,
        preview_features,
        extension_types,
    )
    .await?;
    let to = json_rpc_diff_target_to_dialect(
        &params.to,
        datasource_urls,
        namespaces,
        &filter,
        preview_features,
        extension_types,
    )
    .await?;

    // The `from` connector takes precedence, because if we think of diffs as migrations, `from` is
    // the target where the migration would be applied.
    //
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

/// Refuses a shadow database that is one of the databases the diff looks at.
///
/// A migrations target replays the migration history into the shadow database, which is reset
/// first. When the shadow database is in fact the datasource's own database, or the database on the
/// other end of the diff, replaying the history destroys the data that the diff was meant to
/// describe — and it does so silently, since diffing a database against a copy of the migration
/// history that was just written into it yields no difference at all.
///
/// Resolving a diff target is what opens the connection that does the damage, so the check has to
/// happen before any target is resolved.
fn validate_shadow_database_is_not_diffed(params: &DiffParams, datasource_urls: &DatasourceUrls) -> CoreResult<()> {
    let Some(shadow_database_url) = datasource_urls.shadow_database_url.as_deref() else {
        return Ok(());
    };

    let targets = [&params.from, &params.to];

    // Only a migrations target writes to the external shadow database, and its `migration_lock.toml`
    // is what says how the connection strings are to be interpreted.
    let Some(flavour) = targets.iter().find_map(|target| match target {
        DiffTarget::Migrations(migrations) => flavour_from_lock_file(migrations),
        _ => None,
    }) else {
        return Ok(());
    };

    let diffed_urls = datasource_urls
        .url
        .as_deref()
        .into_iter()
        .chain(targets.iter().filter_map(|target| match target {
            DiffTarget::Url(UrlContainer { url }) => Some(url.as_str()),
            _ => None,
        }));

    for url in diffed_urls {
        if sql_schema_connector::urls_denote_same_database(flavour, url, shadow_database_url) {
            return Err(CoreError::user_facing(ShadowDbSameAsMainDb));
        }
    }

    Ok(())
}

fn flavour_from_lock_file(migrations: &MigrationList) -> Option<Flavour> {
    let provider = schema_connector::migrations_directory::read_provider_from_lock_file(&migrations.lockfile)?;

    BUILTIN_CONNECTORS
        .iter()
        .find(|connector| connector.is_provider(&provider))
        .map(|connector| connector.flavour())
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
    datasource_urls: &DatasourceUrls,
    namespaces: Option<Namespaces>,
    filter: &SchemaFilter,
    preview_features: BitFlags<psl::PreviewFeature>,
    extension_types: &dyn ExtensionTypes,
) -> CoreResult<Option<(Box<dyn SchemaDialect>, DatabaseSchema)>> {
    match target {
        DiffTarget::Empty => Ok(None),
        DiffTarget::SchemaDatasource(schemas) => {
            let config_dir = std::path::Path::new(&schemas.config_dir);
            let sources: Vec<_> = schemas.to_psl_input();

            // actually, just use the given `connector`. Verify that the provider is the same
            // as the one assumed by the connector.

            let mut connector = crate::schema_to_connector(&sources, datasource_urls, Some(config_dir))?;
            connector.ensure_connection_validity().await?;
            connector.set_preview_features(preview_features);
            filter.validate(&*connector.schema_dialect())?;

            let schema = connector.schema_from_database(namespaces).await?;
            Ok(Some((connector.schema_dialect(), schema)))
        }
        DiffTarget::SchemaDatamodel(schemas) => {
            let sources = schemas.to_psl_input();

            // Connector only needed to infer the default namespace.
            // If connector cannot be created (e.g. due to invalid or missing URL) we use the dialect's default namespace.
            let (default_namespace, dialect) = match crate::schema_to_connector(&sources, datasource_urls, None) {
                Ok(connector) => (
                    connector.default_runtime_namespace().map(|ns| ns.to_string()),
                    connector.schema_dialect(),
                ),
                Err(_) => {
                    let dialect = crate::schema_to_dialect(&sources)?;
                    (dialect.default_namespace().map(|ns| ns.to_string()), dialect)
                }
            };

            filter.validate(&*dialect)?;

            let schema = dialect.schema_from_datamodel(sources, default_namespace.as_deref(), extension_types)?;

            Ok(Some((dialect, schema)))
        }
        DiffTarget::Url(UrlContainer { url }) => {
            // this will not be supported

            let mut connector = crate::connector_for_connection_string(url.clone(), None, BitFlags::empty())?;
            connector.ensure_connection_validity().await?;
            connector.set_preview_features(preview_features);

            let schema = connector.schema_from_database(namespaces).await?;
            let dialect = connector.schema_dialect();
            filter.validate(&*dialect)?;

            connector.dispose().await?;

            Ok(Some((dialect, schema)))
        }
        DiffTarget::Migrations(migration_list) => {
            let provider =
                schema_connector::migrations_directory::read_provider_from_lock_file(&migration_list.lockfile);

            match (provider.as_deref(), datasource_urls.shadow_database_url.as_deref()) {
                (Some(provider), Some(shadow_database_url)) => {
                    let dialect = ::commands::dialect_for_provider(provider)?;
                    let migrations = Migrations::from_migration_list(migration_list);

                    filter.validate(&*dialect)?;

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
                    filter.validate(&*connector.schema_dialect())?;

                    let schema = connector
                        .schema_from_migrations(&migrations, namespaces, filter)
                        .await?;
                    Ok(Some((connector.schema_dialect(), schema)))
                }
                (Some(_), None) => Err(ConnectorError::from_msg(
                    "You must set `datasource.shadowDatabaseUrl` in your `prisma.config.ts` if you want to diff a migrations directory.".to_owned(),
                )),
                (None, _) => Err(ConnectorError::from_msg(
                    "Could not determine the connector from the migrations directory (missing migration_lock.toml)."
                        .to_owned(),
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_rpc::types::{MigrationList, MigrationLockfile, SchemaFilter};

    const MAIN_URL: &str = "postgresql://user:password@localhost:5432/maindb";

    fn migrations_target(provider: &str) -> DiffTarget {
        DiffTarget::Migrations(MigrationList {
            base_dir: "/tmp/migrations".to_owned(),
            lockfile: MigrationLockfile {
                path: "migration_lock.toml".to_owned(),
                content: Some(format!("provider = \"{provider}\"")),
            },
            shadow_db_init_script: String::new(),
            migration_directories: Vec::new(),
        })
    }

    fn url_target(url: &str) -> DiffTarget {
        DiffTarget::Url(UrlContainer { url: url.to_owned() })
    }

    #[track_caller]
    fn validate(from: DiffTarget, to: DiffTarget, urls: DatasourceUrls) -> CoreResult<()> {
        let params = DiffParams {
            from,
            to,
            script: false,
            exit_code: None,
            filters: SchemaFilter::default(),
        };

        validate_shadow_database_is_not_diffed(&params, &urls)
    }

    #[track_caller]
    fn assert_refused(result: CoreResult<()>) {
        let err = result.unwrap_err();
        assert!(
            err.is_user_facing_error::<ShadowDbSameAsMainDb>(),
            "expected the shadow database error, got {err:?}"
        );
    }

    #[test]
    fn a_shadow_database_that_is_the_datasource_database_is_refused() {
        assert_refused(validate(
            migrations_target("postgresql"),
            DiffTarget::Empty,
            DatasourceUrls {
                url: Some(MAIN_URL.to_owned()),
                shadow_database_url: Some(MAIN_URL.to_owned()),
            },
        ));
    }

    #[test]
    fn a_differently_spelled_shadow_database_url_is_refused() {
        assert_refused(validate(
            migrations_target("postgresql"),
            DiffTarget::Empty,
            DatasourceUrls {
                url: Some(MAIN_URL.to_owned()),
                shadow_database_url: Some("postgres://user:password@LOCALHOST/maindb?schema=shadow".to_owned()),
            },
        ));
    }

    #[test]
    fn a_shadow_database_that_is_a_url_target_is_refused() {
        assert_refused(validate(
            url_target(MAIN_URL),
            migrations_target("postgresql"),
            DatasourceUrls {
                url: None,
                shadow_database_url: Some(MAIN_URL.to_owned()),
            },
        ));
    }

    #[test]
    fn a_separate_shadow_database_is_allowed() {
        validate(
            migrations_target("postgresql"),
            url_target(MAIN_URL),
            DatasourceUrls {
                url: Some(MAIN_URL.to_owned()),
                shadow_database_url: Some("postgresql://user:password@localhost:5432/shadowdb".to_owned()),
            },
        )
        .unwrap();
    }

    #[test]
    fn without_a_migrations_target_the_shadow_database_url_is_not_checked() {
        validate(
            url_target(MAIN_URL),
            DiffTarget::Empty,
            DatasourceUrls {
                url: Some(MAIN_URL.to_owned()),
                shadow_database_url: Some(MAIN_URL.to_owned()),
            },
        )
        .unwrap();
    }

    #[test]
    fn without_a_shadow_database_url_nothing_is_checked() {
        validate(
            migrations_target("postgresql"),
            DiffTarget::Empty,
            DatasourceUrls {
                url: Some(MAIN_URL.to_owned()),
                shadow_database_url: None,
            },
        )
        .unwrap();
    }

    #[test]
    fn an_unresolvable_provider_leaves_the_connection_strings_uncompared() {
        // A migration history whose provider is not a connector we know gives no flavour to compare
        // the connection strings with. Reporting that is the diff command's own job, further down.
        validate(
            migrations_target("not-a-provider"),
            DiffTarget::Empty,
            DatasourceUrls {
                url: Some(MAIN_URL.to_owned()),
                shadow_database_url: Some("postgres://user:password@LOCALHOST/maindb".to_owned()),
            },
        )
        .unwrap();
    }
}
