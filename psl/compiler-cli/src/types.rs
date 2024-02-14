pub use schema_ast::ast;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ParserDatabase {
    ast: ast::SchemaAst,
}
