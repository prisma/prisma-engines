pub(crate) mod expression;
mod into_bson;
pub(crate) mod into_expression;
pub(crate) mod into_operation;
pub(crate) mod operation;

use super::*;
use crate::*;

use crate::root_queries::update::expression::MergedSet;
use crate::root_queries::update::operation::UpdateOperation;
use connector_interface::{FieldPath, WriteOperation};
use into_expression::IntoUpdateExpressions;
use into_operation::IntoUpdateOperation;
use mongodb::bson::{doc, Document};

pub(crate) trait IntoUpdateDocumentExtension {
    fn into_update_docs(self, field: &Field, path: FieldPath) -> crate::Result<Vec<UpdateOperation>>;
}
