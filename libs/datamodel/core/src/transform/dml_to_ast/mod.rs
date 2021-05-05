mod datasource_serializer;
mod generator_serializer;
mod lower;

pub use datasource_serializer::DatasourceSerializer;
pub use generator_serializer::GeneratorSerializer;
pub use lower::LowerDmlToAst;

use crate::{ast, StringFromEnvVar};

fn lower_string_from_env_var(arg_name: &str, string_from_env: &StringFromEnvVar) -> ast::Argument {
    match string_from_env.as_env_var() {
        Some(ref env_var) => {
            let values = vec![ast::Expression::StringValue(env_var.to_string(), ast::Span::empty())];
            ast::Argument::new_function(arg_name, "env", values)
        }
        None => ast::Argument::new_string(arg_name, string_from_env.as_literal().unwrap()),
    }
}
