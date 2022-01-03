use super::super::helpers::*;
use crate::{
    ast::{self, WithSpan},
    common::preview_features::GENERATOR,
    configuration::Generator,
    diagnostics::*,
    transform::ast_to_dml::common::parse_and_validate_preview_features,
    StringFromEnvVar,
};
use std::{collections::HashMap, convert::TryFrom};

const PROVIDER_KEY: &str = "provider";
const OUTPUT_KEY: &str = "output";
const BINARY_TARGETS_KEY: &str = "binaryTargets";
const EXPERIMENTAL_FEATURES_KEY: &str = "experimentalFeatures";
const PREVIEW_FEATURES_KEY: &str = "previewFeatures";
const FIRST_CLASS_PROPERTIES: &[&str] = &[
    PROVIDER_KEY,
    OUTPUT_KEY,
    BINARY_TARGETS_KEY,
    EXPERIMENTAL_FEATURES_KEY,
    PREVIEW_FEATURES_KEY,
];

/// Is responsible for loading and validating Generators defined in an AST.
pub(crate) struct GeneratorLoader;

impl GeneratorLoader {
    pub fn load_generators_from_ast(ast_schema: &ast::SchemaAst, diagnostics: &mut Diagnostics) -> Vec<Generator> {
        let mut generators: Vec<Generator> = Vec::new();

        for gen in ast_schema.generators() {
            if let Some(generator) = Self::lift_generator(gen, diagnostics) {
                generators.push(generator)
            }
        }

        generators
    }

    fn lift_generator(ast_generator: &ast::GeneratorConfig, diagnostics: &mut Diagnostics) -> Option<Generator> {
        let args: HashMap<_, _> = ast_generator
            .properties
            .iter()
            .map(|arg| (arg.name.name.as_str(), ValueValidator::new(&arg.value)))
            .collect();

        let provider = match args.get(PROVIDER_KEY) {
            Some(val) => match StringFromEnvVar::try_from(val.value) {
                Ok(val) => val,
                Err(err) => {
                    diagnostics.push_error(err);
                    return None;
                }
            },
            None => {
                diagnostics.push_error(DatamodelError::new_generator_argument_not_found_error(
                    PROVIDER_KEY,
                    &ast_generator.name.name,
                    *ast_generator.span(),
                ));
                return None;
            }
        };

        let output = match args.get(OUTPUT_KEY).map(|v| StringFromEnvVar::try_from(v.value)) {
            Some(Ok(val)) => Some(val),
            Some(Err(err)) => {
                diagnostics.push_error(err);
                None
            }
            None => None,
        };

        let mut properties: HashMap<String, String> = HashMap::new();

        let binary_targets = match args.get(BINARY_TARGETS_KEY).map(|value_validator| {
            value_validator
                .as_array()
                .iter()
                .map(|v| StringFromEnvVar::try_from(v.value))
                .collect()
        }) {
            Some(Ok(val)) => val,
            Some(Err(err)) => {
                diagnostics.push_error(err);
                Vec::new()
            }
            None => Vec::new(),
        };

        // for compatibility reasons we still accept the old experimental key
        let preview_features_arg = args
            .get(PREVIEW_FEATURES_KEY)
            .or_else(|| args.get(EXPERIMENTAL_FEATURES_KEY))
            .map(|v| (v.as_array().to_str_vec(), v.span()));

        let preview_features = match preview_features_arg {
            Some((Ok(arr), span)) => {
                let (features, mut diag) = parse_and_validate_preview_features(arr, &GENERATOR, span);
                diagnostics.append(&mut diag);

                Some(features)
            }
            Some((Err(err), _)) => {
                diagnostics.push_error(err);
                None
            }
            None => None,
        };

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
                ast::Expression::FieldWithArgs(_, _, _) => String::from("(field with args)"),
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
            documentation: ast_generator.documentation.clone().map(|comment| comment.text),
        })
    }
}
