use crate::{
    ast::WithSpan,
    common::{FeatureMapWithProvider, PreviewFeature, RenamedFeature},
    configuration::{Generator, GeneratorConfigValue, StringFromEnvVar},
    diagnostics::*,
};
use enumflags2::BitFlags;
use itertools::Itertools;
use parser_database::{
    ast::{self, Expression, WithDocumentation},
    coerce, coerce_array,
};
use schema_ast::ast::WithName;
use std::collections::HashMap;

const PROVIDER_KEY: &str = "provider";
const OUTPUT_KEY: &str = "output";
const BINARY_TARGETS_KEY: &str = "binaryTargets";
const PREVIEW_FEATURES_KEY: &str = "previewFeatures";
const ENGINE_TYPE_KEY: &str = "engineType";

const FIRST_CLASS_PROPERTIES: &[&str] = &[PROVIDER_KEY, OUTPUT_KEY, BINARY_TARGETS_KEY, PREVIEW_FEATURES_KEY];

/// Load and validate Generators defined in an AST.
pub(crate) fn load_generators_from_ast(
    ast_schema: &ast::SchemaAst,
    diagnostics: &mut Diagnostics,
    feature_map_with_provider: &FeatureMapWithProvider<'_>,
) -> Vec<Generator> {
    let mut generators: Vec<Generator> = Vec::new();

    for gen in ast_schema.generators() {
        if let Some(generator) = lift_generator(gen, diagnostics, feature_map_with_provider) {
            generators.push(generator);
        }
    }

    generators
}

fn lift_generator(
    ast_generator: &ast::GeneratorConfig,
    diagnostics: &mut Diagnostics,
    feature_map_with_provider: &FeatureMapWithProvider<'_>,
) -> Option<Generator> {
    let generator_name = ast_generator.name.name.as_str();
    let args: HashMap<_, &Expression> = ast_generator
        .properties
        .iter()
        .map(|arg| match &arg.value {
            Some(expr) => Some((arg.name(), expr)),
            None => {
                diagnostics.push_error(DatamodelError::new_config_property_missing_value_error(
                    arg.name(),
                    generator_name,
                    "generator",
                    ast_generator.span,
                ));

                None
            }
        })
        .collect::<Option<HashMap<_, _>>>()?;

    // E.g., "library"
    if let Some(expr) = args.get(ENGINE_TYPE_KEY) {
        if !expr.is_string() {
            diagnostics.push_error(DatamodelError::new_type_mismatch_error(
                "String",
                expr.describe_value_type(),
                &expr.to_string(),
                expr.span(),
            ))
        }
    }

    // E.g., "prisma-client-js"
    let provider = match args.get(PROVIDER_KEY) {
        Some(val) => StringFromEnvVar::coerce(val, diagnostics)?,
        None => {
            diagnostics.push_error(DatamodelError::new_generator_argument_not_found_error(
                PROVIDER_KEY,
                &ast_generator.name.name,
                ast_generator.span(),
            ));
            return None;
        }
    };

    let output = args
        .get(OUTPUT_KEY)
        .and_then(|v| StringFromEnvVar::coerce(v, diagnostics));

    let mut properties = HashMap::new();

    let binary_targets = args
        .get(BINARY_TARGETS_KEY)
        .and_then(|arg| coerce_array(arg, &StringFromEnvVar::coerce, diagnostics))
        .unwrap_or_default();

    // for compatibility reasons we still accept the old experimental key
    let preview_features = args
        .get(PREVIEW_FEATURES_KEY)
        .and_then(|v| coerce_array(v, &coerce::string, diagnostics).map(|arr| (arr, v.span())))
        .map(|(arr, span)| parse_and_validate_preview_features(arr, feature_map_with_provider, span, diagnostics));

    for prop in &ast_generator.properties {
        let is_first_class_prop = FIRST_CLASS_PROPERTIES.iter().any(|k| *k == prop.name());
        if is_first_class_prop {
            continue;
        }

        let value = match &prop.value {
            Some(val) => GeneratorConfigValue::from(val),
            None => {
                diagnostics.push_error(DatamodelError::new_config_property_missing_value_error(
                    prop.name(),
                    generator_name,
                    "generator",
                    prop.span,
                ));
                continue;
            }
        };

        properties.insert(prop.name().to_owned(), value);
    }

    Some(Generator {
        name: ast_generator.name.name.clone(),
        provider,
        output,
        binary_targets,
        preview_features,
        config: properties,
        documentation: ast_generator.documentation().map(String::from),
        span: ast_generator.span,
    })
}

fn parse_and_validate_preview_features(
    preview_features: Vec<&str>,
    feature_map_with_provider: &FeatureMapWithProvider<'_>,
    span: ast::Span,
    diagnostics: &mut Diagnostics,
) -> BitFlags<PreviewFeature> {
    let mut features = BitFlags::empty();

    for feature_str in preview_features {
        let feature_opt = PreviewFeature::parse_opt(feature_str);
        match feature_opt {
            Some(feature) if feature_map_with_provider.is_deprecated(feature) => {
                match feature_map_with_provider.is_renamed(feature) {
                    Some(RenamedFeature::AllProviders(renamed_feature)) => {
                        features |= renamed_feature.to;

                        diagnostics.push_warning(DatamodelWarning::new_preview_feature_renamed(
                            feature_str,
                            renamed_feature.to,
                            renamed_feature.prisly_link_endpoint,
                            span,
                        ));
                    }
                    Some(RenamedFeature::ForProvider((provider, renamed_feature))) => {
                        features |= renamed_feature.to;

                        diagnostics.push_warning(DatamodelWarning::new_preview_feature_renamed_for_provider(
                            provider,
                            feature_str,
                            renamed_feature.to,
                            renamed_feature.prisly_link_endpoint,
                            span,
                        ));
                    }
                    None => {
                        features |= feature;
                        diagnostics.push_warning(DatamodelWarning::new_preview_feature_deprecated(feature_str, span));
                    }
                }
            }

            Some(feature) if !feature_map_with_provider.is_valid(feature) => {
                diagnostics.push_error(DatamodelError::new_preview_feature_not_known_error(
                    feature_str,
                    feature_map_with_provider
                        .active_features()
                        .iter()
                        .map(|pf| pf.to_string())
                        .join(", "),
                    span,
                ))
            }

            Some(feature) => features |= feature,

            None => diagnostics.push_error(DatamodelError::new_preview_feature_not_known_error(
                feature_str,
                feature_map_with_provider
                    .active_features()
                    .iter()
                    .map(|pf| pf.to_string())
                    .join(", "),
                span,
            )),
        }
    }

    features
}
