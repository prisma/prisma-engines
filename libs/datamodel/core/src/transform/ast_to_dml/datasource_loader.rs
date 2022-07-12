use crate::{
    ast::{self, SourceConfig, Span},
    common::preview_features::PreviewFeature,
    configuration::StringFromEnvVar,
    diagnostics::{DatamodelError, Diagnostics},
    parser_database::{ValueListValidator, ValueValidator},
    Datasource,
};
use datamodel_connector::ReferentialIntegrity;
use enumflags2::BitFlags;
use mongodb_datamodel_connector::*;
use sql_datamodel_connector::*;
use std::{borrow::Cow, collections::HashMap, convert::TryFrom};

const PREVIEW_FEATURES_KEY: &str = "previewFeatures";
const SHADOW_DATABASE_URL_KEY: &str = "shadowDatabaseUrl";
const URL_KEY: &str = "url";

/// Is responsible for loading and validating Datasources defined in an AST.
pub(crate) struct DatasourceLoader;

impl DatasourceLoader {
    /// Loads all datasources from the provided schema AST.
    /// - `ignore_datasource_urls`: datasource URLs are not parsed. They are replaced with dummy values.
    /// - `datasource_url_overrides`: datasource URLs are not parsed and overridden with the provided ones.
    pub fn load_datasources_from_ast(
        &self,
        ast_schema: &ast::SchemaAst,
        preview_features: BitFlags<PreviewFeature>,
        diagnostics: &mut Diagnostics,
    ) -> Vec<Datasource> {
        let mut sources = Vec::new();

        for src in ast_schema.sources() {
            if let Some(source) = self.lift_datasource(src, preview_features, diagnostics) {
                sources.push(source)
            }
        }

        if sources.len() > 1 {
            for src in ast_schema.sources() {
                diagnostics.push_error(DatamodelError::new_source_validation_error(
                    "You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet.",
                    &src.name.name,
                    src.span,
                ));
            }
        }

        sources
    }

    fn lift_datasource(
        &self,
        ast_source: &ast::SourceConfig,
        preview_features: BitFlags<PreviewFeature>,
        diagnostics: &mut Diagnostics,
    ) -> Option<Datasource> {
        let source_name = &ast_source.name.name;
        let mut args: HashMap<_, _> = ast_source
            .properties
            .iter()
            .map(|arg| (arg.name.name.as_str(), (arg.span, ValueValidator::new(&arg.value))))
            .collect();

        let (_, provider_arg) = match args.remove("provider") {
            Some(provider) => provider,
            None => {
                diagnostics.push_error(DatamodelError::new_source_argument_not_found_error(
                    "provider",
                    &ast_source.name.name,
                    ast_source.span,
                ));
                return None;
            }
        };

        if provider_arg.is_from_env() {
            let msg = Cow::Borrowed("A datasource must not use the env() function in the provider argument.");
            diagnostics.push_error(DatamodelError::new_functional_evaluation_error(msg, ast_source.span));
            return None;
        }

        let provider = match provider_arg.as_string_literal() {
            Some(("", _)) => {
                diagnostics.push_error(DatamodelError::new_source_validation_error(
                    "The provider argument in a datasource must not be empty",
                    source_name,
                    provider_arg.span(),
                ));
                return None;
            }
            None => {
                diagnostics.push_error(DatamodelError::new_source_validation_error(
                    "The provider argument in a datasource must be a string literal",
                    source_name,
                    provider_arg.span(),
                ));
                return None;
            }
            Some((provider, _)) => provider,
        };

        let (_, url_arg) = match args.remove(URL_KEY) {
            Some(url_arg) => url_arg,
            None => {
                diagnostics.push_error(DatamodelError::new_source_argument_not_found_error(
                    URL_KEY,
                    &ast_source.name.name,
                    ast_source.span,
                ));
                return None;
            }
        };

        let url = match StringFromEnvVar::try_from(url_arg.value) {
            Ok(str_from_env_var) => str_from_env_var,
            Err(err) => {
                diagnostics.push_error(err);
                return None;
            }
        };

        let shadow_database_url_arg = args.remove(SHADOW_DATABASE_URL_KEY);

        let shadow_database_url: Option<(StringFromEnvVar, Span)> =
            if let Some((_, shadow_database_url_arg)) = shadow_database_url_arg.as_ref() {
                match StringFromEnvVar::try_from(shadow_database_url_arg.value) {
                    Ok(shadow_database_url) => Some(shadow_database_url)
                        .filter(|s| !s.as_literal().map(|lit| lit.is_empty()).unwrap_or(false))
                        .map(|url| (url, shadow_database_url_arg.span())),
                    Err(err) => {
                        diagnostics.push_error(err);
                        None
                    }
                }
            } else {
                None
            };

        preview_features_guardrail(&args, diagnostics);

        let documentation = ast_source.documentation.as_ref().map(|comment| comment.text.clone());
        let referential_integrity = get_referential_integrity(&args, preview_features, ast_source, diagnostics);

        let active_connector: &'static dyn datamodel_connector::Connector = match provider {
            p if MYSQL.is_provider(p) => MYSQL,
            p if POSTGRES.is_provider(p) => POSTGRES,
            p if SQLITE.is_provider(p) => SQLITE,
            p if MSSQL.is_provider(p) => MSSQL,
            p if MONGODB.is_provider(p) => MONGODB,
            p if COCKROACH.is_provider(p) => COCKROACH,

            _ => {
                diagnostics.push_error(DatamodelError::new_datasource_provider_not_known_error(
                    provider,
                    provider_arg.span(),
                ));

                return None;
            }
        };

        if let Some(integrity) = referential_integrity {
            if !active_connector
                .allowed_referential_integrity_settings()
                .contains(integrity)
            {
                let span = args
                    .get("referentialIntegrity")
                    .map(|(_, v)| v.span())
                    .unwrap_or_else(Span::empty);

                let supported_values = active_connector
                    .allowed_referential_integrity_settings()
                    .iter()
                    .map(|v| format!(r#""{}""#, v))
                    .collect::<Vec<_>>()
                    .join(", ");

                let message = format!(
                    "Invalid referential integrity setting: \"{}\". Supported values: {}",
                    integrity, supported_values,
                );

                let error = DatamodelError::new_source_validation_error(&message, "referentialIntegrity", span);

                diagnostics.push_error(error);
            }
        }

        // we handle these elsewhere
        let _ = args.remove("previewFeatures");
        let _ = args.remove("referentialIntegrity");
        for (name, (span, _)) in args.into_iter() {
            diagnostics.push_error(DatamodelError::new_property_not_known_error(name, span));
        }

        Some(Datasource {
            name: source_name.to_string(),
            provider: provider.to_owned(),
            active_provider: active_connector.provider_name(),
            url,
            url_span: url_arg.span(),
            documentation,
            active_connector,
            shadow_database_url,
            referential_integrity,
        })
    }
}

const REFERENTIAL_INTEGRITY_PREVIEW_FEATURE_ERR: &str = r#"
The `referentialIntegrity` option can only be set if the preview feature is enabled in a generator block.

Example:

generator client {
    provider = "prisma-client-js"
    previewFeatures = ["referentialIntegrity"]
}
"#;

fn get_referential_integrity(
    args: &HashMap<&str, (Span, ValueValidator<'_>)>,
    preview_features: BitFlags<PreviewFeature>,
    source: &SourceConfig,
    diagnostics: &mut Diagnostics,
) -> Option<ReferentialIntegrity> {
    args.get("referentialIntegrity").and_then(|(span, value)| {
        if !preview_features.contains(PreviewFeature::ReferentialIntegrity) {
            diagnostics.push_error(DatamodelError::new_source_validation_error(
                REFERENTIAL_INTEGRITY_PREVIEW_FEATURE_ERR,
                &source.name.name,
                *span,
            ));

            None
        } else {
            match value.as_str() {
                Ok("prisma") => Some(ReferentialIntegrity::Prisma),
                Ok("foreignKeys") => Some(ReferentialIntegrity::ForeignKeys),
                Ok(s) => {
                    let message = format!(
                        "Invalid referential integrity setting: \"{}\". Supported values: \"prisma\", \"foreignKeys\"",
                        s
                    );

                    let error =
                        DatamodelError::new_source_validation_error(&message, "referentialIntegrity", value.span());

                    diagnostics.push_error(error);

                    None
                }
                Err(e) => {
                    diagnostics.push_error(e);
                    None
                }
            }
        }
    })
}

fn preview_features_guardrail(args: &HashMap<&str, (Span, ValueValidator<'_>)>, diagnostics: &mut Diagnostics) {
    let arg = args.get(PREVIEW_FEATURES_KEY);

    if let Some(val) = arg {
        let span = val.0;
        if let Ok(features) = val.1.as_array().to_str_vec() {
            if features.is_empty() {
                return;
            }
        }
        let msg = "Preview features are only supported in the generator block. Please move this field to the generator block.";
        diagnostics.push_error(DatamodelError::new(std::borrow::Cow::Borrowed(msg), span));
    }
}
