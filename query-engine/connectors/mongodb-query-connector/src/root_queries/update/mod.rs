mod expression;
mod into_bson;
mod into_expression;
mod utils;

use super::*;
use crate::*;
use connector_interface::{FieldPath, WriteOperation};
use into_expression::IntoUpdateExpressionExtension;
use mongodb::bson::Document;

pub(crate) trait IntoUpdateDocumentExtension {
    fn into_update_docs(self, field: &Field, path: FieldPath) -> crate::Result<Vec<Document>>;
}

impl IntoUpdateDocumentExtension for WriteOperation {
    fn into_update_docs(self, field: &Field, path: FieldPath) -> crate::Result<Vec<Document>> {
        let expressions = self.into_update_expressions(field, path)?;
        let mut documents = vec![];

        for expr in expressions {
            documents.extend(utils::flatten_bson(expr.into_bson()?)?);
        }

        Ok(documents)
    }
}
