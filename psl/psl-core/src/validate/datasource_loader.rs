use crate::{
    ast::{self, SourceConfig, Span},
    configuration::StringFromEnvVar,
    datamodel_connector::RelationMode,
    diagnostics::{DatamodelError, Diagnostics},
    Datasource, PreviewFeature,
};
use enumflags2::BitFlags;
use parser_database::{ast::WithDocumentation, coerce, coerce_array, coerce_opt};
use std::{borrow::Cow, collections::HashMap};

const PREVIEW_FEATURES_KEY: &str = "previewFeatures";
const SCHEMAS_KEY: &str = "schemas";
const SHADOW_DATABASE_URL_KEY: &str = "shadowDatabaseUrl";
const URL_KEY: &str = "url";

/// Loads all datasources from the provided schema AST.
/// - `ignore_datasource_urls`: datasource URLs are not parsed. They are replaced with dummy values.
/// - `datasource_url_overrides`: datasource URLs are not parsed and overridden with the provided ones.
pub(crate) fn load_datasources_from_ast(
    ast_schema: &ast::SchemaAst,
    preview_features: BitFlags<PreviewFeature>,
    diagnostics: &mut Diagnostics,
    connectors: crate::ConnectorRegistry,
) -> Vec<Datasource> {
    let mut sources = Vec::new();

    for src in ast_schema.sources() {
        if let Some(source) = lift_datasource(src, preview_features, diagnostics, connectors) {
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
    ast_source: &ast::SourceConfig,
    preview_features: BitFlags<PreviewFeature>,
    diagnostics: &mut Diagnostics,
    connectors: crate::ConnectorRegistry,
) -> Option<Datasource> {
    let source_name = &ast_source.name.name;
    let mut args: HashMap<_, _> = ast_source
        .properties
        .iter()
        .map(|arg| (arg.name.name.as_str(), (arg.span, &arg.value)))
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

    if provider_arg.is_env_expression() {
        let msg = Cow::Borrowed("A datasource must not use the env() function in the provider argument.");
        diagnostics.push_error(DatamodelError::new_functional_evaluation_error(msg, ast_source.span));
        return None;
    }

    let provider = match coerce_opt::string(provider_arg) {
        Some("") => {
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
        Some(provider) => provider,
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

    let url = StringFromEnvVar::coerce(url_arg, diagnostics)?;
    let shadow_database_url_arg = args.remove(SHADOW_DATABASE_URL_KEY);

    let shadow_database_url: Option<(StringFromEnvVar, Span)> =
        if let Some((_, shadow_database_url_arg)) = shadow_database_url_arg.as_ref() {
            match StringFromEnvVar::coerce(shadow_database_url_arg, diagnostics) {
                Some(shadow_database_url) => Some(shadow_database_url)
                    .filter(|s| !s.as_literal().map(|lit| lit.is_empty()).unwrap_or(false))
                    .map(|url| (url, shadow_database_url_arg.span())),
                None => None,
            }
        } else {
            None
        };

    preview_features_guardrail(&mut args, diagnostics);

    let documentation = ast_source.documentation().map(String::from);

    let active_connector: &'static dyn crate::datamodel_connector::Connector =
        match connectors.iter().find(|c| c.is_provider(provider)) {
            Some(c) => *c,
            None => {
                diagnostics.push_error(DatamodelError::new_datasource_provider_not_known_error(
                    provider,
                    provider_arg.span(),
                ));

                return None;
            }
        };

    let relation_mode = get_relation_mode(&mut args, preview_features, ast_source, diagnostics, active_connector);

    let (schemas, schemas_span) = args
        .remove(SCHEMAS_KEY)
        .and_then(|(_, expr)| coerce_array(expr, &coerce::string_with_span, diagnostics).map(|b| (b, expr.span())))
        .map(|(mut schemas, span)| {
            if schemas.is_empty() {
                let error = DatamodelError::new_schemas_array_empty_error(span);

                diagnostics.push_error(error);
            }

            schemas.sort_by(|(a, _), (b, _)| a.cmp(b));

            for pair in schemas.windows(2) {
                if pair[0].0 == pair[1].0 {
                    diagnostics.push_error(DatamodelError::new_static(
                        "Duplicated schema names are not allowed",
                        pair[0].1,
                    ))
                }
            }

            (schemas, Some(span))
        })
        .unwrap_or_default();

    let connector_data = active_connector.parse_datasource_properties(&mut args, diagnostics);

    for (name, (span, _)) in args.into_iter() {
        diagnostics.push_error(DatamodelError::new_property_not_known_error(name, span));
    }

    Some(Datasource {
        namespaces: schemas.into_iter().map(|(s, span)| (s.to_owned(), span)).collect(),
        schemas_span,
        name: source_name.to_string(),
        provider: provider.to_owned(),
        active_provider: active_connector.provider_name(),
        url,
        url_span: url_arg.span(),
        documentation,
        active_connector,
        shadow_database_url,
        relation_mode,
        connector_data,
    })
}

fn get_relation_mode(
    args: &mut HashMap<&str, (Span, &ast::Expression)>,
    preview_features: BitFlags<PreviewFeature>,
    source: &SourceConfig,
    diagnostics: &mut Diagnostics,
    connector: &'static dyn crate::datamodel_connector::Connector,
) -> Option<RelationMode> {
    match (args.remove("relationMode"), args.remove("referentialIntegrity")) {
        (None, None) => None,
        (Some(_), Some((span, _))) => {
            diagnostics.push_error(DatamodelError::new_referential_integrity_and_relation_mode_cooccur_error(span));
            None
        }
        (Some((_span, rm)), None) | (None, Some((_span, rm))) => {
            let integrity = match coerce::string(rm, diagnostics)? {
                "prisma" => RelationMode::Prisma,
                "foreignKeys" => RelationMode::ForeignKeys,
                other => {
                    let message = format!(
                        "Invalid relation mode setting: \"{other}\". Supported values: \"prisma\", \"foreignKeys\"",
                    );
                    let error = DatamodelError::new_source_validation_error(&message, "relationMode", source.span);
                    diagnostics.push_error(error);
                    return None;
                }
            };

            if !connector.allowed_relation_mode_settings().contains(integrity) {
                let supported_values = connector
                    .allowed_relation_mode_settings()
                    .iter()
                    .map(|v| format!(r#""{}""#, v))
                    .collect::<Vec<_>>()
                    .join(", ");

                let message = format!(
                    "Invalid relation mode setting: \"{integrity}\". Supported values: {supported_values}",
                );
                let error = DatamodelError::new_source_validation_error(&message, "relationMode", rm.span());
                diagnostics.push_error(error);
            }

            Some(integrity)
        }
    }
}

fn preview_features_guardrail(args: &mut HashMap<&str, (Span, &ast::Expression)>, diagnostics: &mut Diagnostics) {
    if let Some((span, _)) = args.remove(PREVIEW_FEATURES_KEY) {
        let msg = "Preview features are only supported in the generator block. Please move this field to the generator block.";
        diagnostics.push_error(DatamodelError::new_static(msg, span));
    }
}
