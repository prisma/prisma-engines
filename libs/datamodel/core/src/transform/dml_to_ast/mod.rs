mod datasource_serializer;
mod generator_serializer;
mod lower;
mod lower_enum_attributes;
mod lower_enum_value_attributes;
mod lower_field;
mod lower_model_attributes;

pub use datasource_serializer::add_sources_to_ast;
pub use generator_serializer::GeneratorSerializer;
pub use lower::LowerDmlToAst;

use crate::{ast, configuration::StringFromEnvVar};

fn lower_string_from_env_var(arg_name: &str, string_from_env: &StringFromEnvVar) -> ast::ConfigBlockProperty {
    match string_from_env.as_env_var() {
        Some(ref env_var) => {
            let values = vec![ast::Expression::StringValue(env_var.to_string(), ast::Span::empty())];
            ast::ConfigBlockProperty {
                name: ast::Identifier::new(arg_name),
                value: ast::Expression::Function("env".to_owned(), values, ast::Span::empty()),
                span: ast::Span::empty(),
            }
        }
        None => ast::ConfigBlockProperty {
            name: ast::Identifier::new(arg_name),
            value: ast::Expression::StringValue(string_from_env.as_literal().unwrap().to_string(), ast::Span::empty()),
            span: ast::Span::empty(),
        },
    }
}
