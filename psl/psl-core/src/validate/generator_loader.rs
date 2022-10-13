use crate::{
    ast::WithSpan,
    common::{FeatureMap, PreviewFeature, ALL_PREVIEW_FEATURES},
    configuration::{Generator, StringFromEnvVar},
    diagnostics::*,
};
use enumflags2::BitFlags;
use itertools::Itertools;
use parser_database::{
    ast::{self, WithDocumentation},
    coerce, coerce_array,
};
use std::collections::HashMap;

const PROVIDER_KEY: &str = "provider";
const OUTPUT_KEY: &str = "output";
const BINARY_TARGETS_KEY: &str = "binaryTargets";
const EXPERIMENTAL_FEATURES_KEY: &str = "experimentalFeatures";
const PREVIEW_FEATURES_KEY: &str = "previewFeatures";
const ENGINE_TYPE_KEY: &str = "engineType";

const FIRST_CLASS_PROPERTIES: &[&str] = &[
    PROVIDER_KEY,
    OUTPUT_KEY,
    BINARY_TARGETS_KEY,
    EXPERIMENTAL_FEATURES_KEY,
    PREVIEW_FEATURES_KEY,
];

/// Load and validate Generators defined in an AST.
pub(crate) fn load_generators_from_ast(ast_schema: &ast::SchemaAst, diagnostics: &mut Diagnostics) -> Vec<Generator> {
    let mut generators: Vec<Generator> = Vec::new();

    for gen in ast_schema.generators() {
        if let Some(generator) = lift_generator(gen, diagnostics) {
            generators.push(generator)
        }
    }

    generators
}

fn lift_generator(ast_generator: &ast::GeneratorConfig, diagnostics: &mut Diagnostics) -> Option<Generator> {
    let args: HashMap<_, _> = ast_generator
        .properties
        .iter()
        .map(|arg| (arg.name.name.as_str(), &arg.value))
        .collect();

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

    let mut properties: HashMap<String, String> = HashMap::new();

    let binary_targets = args
        .get(BINARY_TARGETS_KEY)
        .and_then(|arg| coerce_array(arg, &StringFromEnvVar::coerce, diagnostics))
        .unwrap_or_default();

    // for compatibility reasons we still accept the old experimental key
    let preview_features = args
        .get(PREVIEW_FEATURES_KEY)
        .or_else(|| args.get(EXPERIMENTAL_FEATURES_KEY))
        .and_then(|v| coerce_array(v, &coerce::string, diagnostics).map(|arr| (arr, v.span())))
        .map(|(arr, span)| parse_and_validate_preview_features(arr, &ALL_PREVIEW_FEATURES, span, diagnostics));

    for prop in &ast_generator.properties {
        let is_first_class_prop = FIRST_CLASS_PROPERTIES.iter().any(|k| *k == prop.name.name);
        if is_first_class_prop {
            continue;
        }

        let value = match &prop.value {
            ast::Expression::NumericValue(val, _) => val.clone(),
            ast::Expression::StringValue(val, _) => val.clone(),
            ast::Expression::ConstantValue(val, _) => val.clone(),
            ast::Expression::Function(_, _, _) => String::from("(function)"),
            ast::Expression::Array(_, _) => String::from("(array)"),
        };

        properties.insert(prop.name.name.clone(), value);
    }

    Some(Generator {
        name: ast_generator.name.name.clone(),
        provider,
        output,
        binary_targets,
        preview_features,
        config: properties,
        documentation: ast_generator.documentation().map(String::from),
    })
}

fn parse_and_validate_preview_features(
    preview_features: Vec<&str>,
    feature_map: &FeatureMap,
    span: ast::Span,
    diagnostics: &mut Diagnostics,
) -> BitFlags<PreviewFeature> {
    let mut features = BitFlags::empty();

    for feature_str in preview_features {
        let feature_opt = PreviewFeature::parse_opt(feature_str);
        match feature_opt {
            Some(feature) if feature_map.is_deprecated(feature) => {
                features |= feature;
                diagnostics.push_warning(DatamodelWarning::new_feature_deprecated(feature_str, span));
            }

            Some(feature) if !feature_map.is_valid(feature) => {
                diagnostics.push_error(DatamodelError::new_preview_feature_not_known_error(
                    feature_str,
                    feature_map.active_features().iter().map(|pf| pf.to_string()).join(", "),
                    span,
                ))
            }

            Some(feature) => features |= feature,

            None => diagnostics.push_error(DatamodelError::new_preview_feature_not_known_error(
                feature_str,
                feature_map.active_features().iter().map(|pf| pf.to_string()).join(", "),
                span,
            )),
        }
    }

    features
}
