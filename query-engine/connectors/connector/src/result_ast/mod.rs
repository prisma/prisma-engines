mod read_result_ast;
mod write_result_ast;

pub use read_result_ast::*;
pub use write_result_ast::*;

use prisma_models::{PrismaValue, GraphqlId};

#[derive(Debug, Clone)]
pub struct ScalarListValues {
    pub record_id: GraphqlId,
    pub values: Vec<PrismaValue>,
}