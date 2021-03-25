mod datasource_serializer;
mod generator_serializer;
mod lower;

use crate::ast;
use crate::StringFromEnvVar;
pub use datasource_serializer::DatasourceSerializer;
pub use generator_serializer::GeneratorSerializer;
pub use lower::LowerDmlToAst;

fn lower_string_from_env_var(string_from_env: &StringFromEnvVar) -> ast::Argument {
    match string_from_env.from_env_var {
        Some(ref env_var) => {
            let values = vec![ast::Expression::StringValue(env_var.to_string(), ast::Span::empty())];
            ast::Argument::new_function(string_from_env.name, "env", values)
        }
        None => ast::Argument::new_string(string_from_env.name, &string_from_env.value),
    }
}
