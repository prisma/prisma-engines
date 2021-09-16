use crate::{ast, configuration::Generator, transform::dml_to_ast::lower_string_from_env_var};

pub struct GeneratorSerializer {}

impl GeneratorSerializer {
    pub fn add_generators_to_ast(generators: &[Generator], ast_datamodel: &mut ast::SchemaAst) {
        let mut tops: Vec<ast::Top> = Vec::new();

        for generator in generators {
            tops.push(ast::Top::Generator(Self::lower_generator(generator)))
        }

        // Do this dance so that generators come before other top elements
        tops.append(&mut ast_datamodel.tops);

        ast_datamodel.tops = tops;
    }

    fn lower_generator(generator: &Generator) -> ast::GeneratorConfig {
        let mut arguments: Vec<ast::Argument> = vec![super::lower_string_from_env_var("provider", &generator.provider)];

        if let Some(output) = &generator.output {
            arguments.push(super::lower_string_from_env_var("output", output));
        }

        if !&generator.preview_features.is_empty() {
            let features: Vec<ast::Expression> = generator
                .preview_features
                .iter()
                .map(|f| ast::Expression::StringValue(f.to_string(), ast::Span::empty()))
                .collect::<Vec<ast::Expression>>();

            arguments.push(ast::Argument::new_array("previewFeatures", features));
        }

        let platform_values: Vec<ast::Expression> = generator
            .binary_targets
            .iter()
            .map(|p| lower_string_from_env_var("binaryTargets", p).value)
            .collect();
        if !platform_values.is_empty() {
            arguments.push(ast::Argument::new_array("binaryTargets", platform_values));
        }

        for (key, value) in &generator.config {
            arguments.push(ast::Argument::new_string(key, value.to_string()));
        }

        ast::GeneratorConfig {
            name: ast::Identifier::new(&generator.name),
            properties: arguments,
            documentation: generator.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        }
    }
}
