mod filters;
mod query_arguments;

pub use filters::*;
pub use query_arguments::*;

use crate::{query_document::*, QueryGraphBuilderResult};
use prisma_models::{ModelIdentifier, PrismaValue, RecordIdentifier};
use std::convert::TryInto;

pub fn extract_identifier(
    value: ParsedInputValue,
    model_id: &ModelIdentifier,
) -> QueryGraphBuilderResult<Option<RecordIdentifier>> {
    if model_id.len() > 1 {
        todo!()
    } else {
        let value: PrismaValue = value.try_into()?;
        match value {
            PrismaValue::Null => Ok(None),
            value => {
                let field = model_id
                    .fields()
                    .nth(0)
                    .map(|f| f.clone())
                    .expect("Expected model identifier to have at least one field.");

                Ok(Some(RecordIdentifier::new(vec![(field, value)])))
            }
        }
    }
}
