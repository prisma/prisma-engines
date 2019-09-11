pub mod read_ast;
pub mod write_ast;

pub use read_ast::*;
pub use write_ast::*;

#[derive(Debug, Clone)]
pub enum Query {
    Read(ReadQuery),
    Write(WriteQuery),
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Read(q) => write!(f, "{}", q),
            Self::Write(WriteQuery::Root(q)) => write!(f, "{}", q),
            _ => unimplemented!(),
        }
    }
}