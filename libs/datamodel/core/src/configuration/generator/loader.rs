use crate::{
    ast,
    common::{argument::Arguments, value::ValueListValidator},
    configuration::Generator,
    error::*,
};
use std::collections::HashMap;

const PROVIDER_KEY: &str = "provider";
const OUTPUT_KEY: &str = "output";
const BINARY_TARGETS_KEY: &str = "binaryTargets";
const FIRST_CLASS_PROPERTIES: &[&str] = &[PROVIDER_KEY, OUTPUT_KEY, BINARY_TARGETS_KEY];

pub struct GeneratorLoader {}

impl GeneratorLoader {
    pub fn load_generators_from_ast(ast_schema: &ast::SchemaAst) -> Result<Vec<Generator>, ErrorCollection> {
        let mut generators: Vec<Generator> = vec![];
        let mut errors = ErrorCollection::new();

        for gen in &ast_schema.generators() {
            match Self::lift_generator(&gen) {
                Ok(loaded_gen) => generators.push(loaded_gen),
                // Lift error.
                Err(DatamodelError::ArgumentNotFound { argument_name, span }) => errors.push(
                    DatamodelError::new_generator_argument_not_found_error(&argument_name, &gen.name.name, span),
                ),
                Err(err) => errors.push(err),
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(generators)
        }
    }

    fn lift_generator(ast_generator: &ast::GeneratorConfig) -> Result<Generator, DatamodelError> {
        let mut args = Arguments::new(&ast_generator.properties, ast_generator.span);

        let provider = args.arg(PROVIDER_KEY)?.as_str()?;
        let output = if let Ok(arg) = args.arg(OUTPUT_KEY) {
            Some(arg.as_str()?)
        } else {
            None
        };

        let mut properties: HashMap<String, String> = HashMap::new();

        let binary_targets = match args.arg(BINARY_TARGETS_KEY).ok() {
            Some(x) => x.as_array()?.to_str_vec()?,
            None => Vec::new(),
        };

        for prop in &ast_generator.properties {
            let is_first_class_prop = FIRST_CLASS_PROPERTIES.iter().any(|k| *k == prop.name.name);
            if is_first_class_prop {
                continue;
            }

            properties.insert(prop.name.name.clone(), prop.value.to_string());
        }

        Ok(Generator {
            name: ast_generator.name.name.clone(),
            provider,
            output,
            binary_targets,
            config: properties,
            documentation: ast_generator.documentation.clone().map(|comment| comment.text),
        })
    }

    pub fn add_generators_to_ast(generators: &[Generator], ast_datamodel: &mut ast::SchemaAst) {
        let mut tops: Vec<ast::Top> = Vec::new();

        for generator in generators {
            tops.push(ast::Top::Generator(Self::lower_generator(&generator)))
        }

        // Prepend generators.
        tops.append(&mut ast_datamodel.tops);

        ast_datamodel.tops = tops;
    }

    fn lower_generator(generator: &Generator) -> ast::GeneratorConfig {
        let mut arguments: Vec<ast::Argument> = Vec::new();

        arguments.push(ast::Argument::new_string("provider", &generator.provider));

        if let Some(output) = &generator.output {
            arguments.push(ast::Argument::new_string("output", &output));
        }

        let platform_values: Vec<ast::Expression> = generator
            .binary_targets
            .iter()
            .map(|p| ast::Expression::StringValue(p.to_string(), ast::Span::empty()))
            .collect();
        if !platform_values.is_empty() {
            arguments.push(ast::Argument::new_array("binaryTargets", platform_values));
        }

        for (key, value) in &generator.config {
            arguments.push(ast::Argument::new_string(&key, &value));
        }

        ast::GeneratorConfig {
            name: ast::Identifier::new(&generator.name),
            properties: arguments,
            documentation: generator.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        }
    }
}
