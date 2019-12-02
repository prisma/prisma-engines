use super::traits::Source;
use crate::ast;

pub struct SourceSerializer {}

impl SourceSerializer {
    pub fn add_sources_to_ast(sources: &[Box<dyn Source + Send + Sync>], ast_datamodel: &mut ast::SchemaAst) {
        let mut tops: Vec<ast::Top> = Vec::new();

        for source in sources {
            tops.push(ast::Top::Source(Self::source_to_ast(&**source)))
        }

        // Prepend sources.
        tops.append(&mut ast_datamodel.tops);

        ast_datamodel.tops = tops;
    }

    fn source_to_ast(source: &dyn Source) -> ast::SourceConfig {
        let mut arguments: Vec<ast::Argument> = Vec::new();

        arguments.push(ast::Argument::new_string("provider", source.connector_type()));
        match source.url().from_env_var {
            Some(ref env_var) => {
                let values = vec![ast::Expression::StringValue(env_var.to_string(), ast::Span::empty())];
                arguments.push(ast::Argument::new_function("url", "env", values));
            }
            None => {
                arguments.push(ast::Argument::new_string("url", &source.url().value));
            }
        }

        ast::SourceConfig {
            name: ast::Identifier::new(source.name()),
            properties: arguments,
            documentation: source.documentation().clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        }
    }
}
