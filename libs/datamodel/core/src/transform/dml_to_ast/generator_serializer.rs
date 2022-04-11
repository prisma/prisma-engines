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
        let mut properties: Vec<ast::ConfigBlockProperty> =
            vec![super::lower_string_from_env_var("provider", &generator.provider)];

        if let Some(output) = &generator.output {
            properties.push(super::lower_string_from_env_var("output", output));
        }

        if let Some(ref features) = &generator.preview_features {
            let features: Vec<ast::Expression> = features
                .iter()
                .map(|f| ast::Expression::StringValue(f.to_string(), ast::Span::empty()))
                .collect::<Vec<ast::Expression>>();

            properties.push(ast::ConfigBlockProperty {
                name: ast::Identifier::new("previewFeatures"),
                value: ast::Expression::Array(features, ast::Span::empty()),
                span: ast::Span::empty(),
            })
        }

        let platform_values: Vec<ast::Expression> = generator
            .binary_targets
            .iter()
            .map(|p| lower_string_from_env_var("binaryTargets", p).value)
            .collect();

        if !platform_values.is_empty() {
            properties.push(ast::ConfigBlockProperty {
                name: ast::Identifier::new("binaryTargets"),
                value: ast::Expression::Array(platform_values, ast::Span::empty()),
                span: ast::Span::empty(),
            });
        }

        for (key, value) in &generator.config {
            properties.push(ast::ConfigBlockProperty {
                name: ast::Identifier::new(key),
                value: ast::Expression::StringValue(value.to_owned(), ast::Span::empty()),
                span: ast::Span::empty(),
            });
        }

        ast::GeneratorConfig {
            name: ast::Identifier::new(&generator.name),
            properties,
            documentation: generator.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        }
    }
}
