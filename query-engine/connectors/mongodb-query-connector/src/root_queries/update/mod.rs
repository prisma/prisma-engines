mod expression;
mod into_bson;
mod into_expression;
mod into_operation;
mod operation;

use super::*;
use crate::*;

use connector_interface::{FieldPath, WriteOperation};
use into_expression::IntoUpdateExpressions;
use into_operation::IntoUpdateOperation;
use bson::Document;

pub(crate) trait IntoUpdateDocumentExtension {
    fn into_update_docs(self, field: &Field, path: FieldPath) -> crate::Result<Vec<Document>>;
}

impl IntoUpdateDocumentExtension for WriteOperation {
    fn into_update_docs(self, field: &Field, path: FieldPath) -> crate::Result<Vec<Document>> {
        let operations = self.into_update_operations(field, path)?;
        let mut expressions = vec![];

        for op in operations {
            expressions.extend(op.into_update_expressions()?);
        }

        let mut documents = vec![];

        for expr in expressions {
            documents.push(expr.into_bson()?.into_document()?)
        }

        Ok(documents)
    }
}
