mod index;
mod model;
mod relation;
mod relation_field;
mod scalar_field;

pub(crate) use index::*;
pub(crate) use model::*;
pub(crate) use relation::*;
pub(crate) use relation_field::*;
pub(crate) use scalar_field::*;

use super::ParserDatabase;
use crate::ast;

impl<'ast> ParserDatabase<'ast> {
    #[track_caller]
    pub(crate) fn walk_model(&self, model_id: ast::ModelId) -> ModelWalker<'ast, '_> {
        ModelWalker {
            model_id,
            db: self,
            model_attributes: &self.types.model_attributes[&model_id],
        }
    }
}
