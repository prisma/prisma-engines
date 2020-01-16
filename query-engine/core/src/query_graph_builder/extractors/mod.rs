mod filters;
mod query_arguments;

pub use filters::*;
pub use query_arguments::*;

use crate::{query_document::*, QueryGraphBuilderResult};
use prisma_models::{Field, ModelIdentifier, PrismaValue, RecordIdentifier};
use std::convert::TryInto;

// pub fn extract_identifier(
//     value: ParsedInputValue,
//     model_id: &ModelIdentifier,
// ) -> QueryGraphBuilderResult<Option<RecordIdentifier>> {
//     // Todo: Ports the old null check to RecordIdentifier. Not entirely sure we still want that.
//     if let Err(_) = value.assert_non_null() {
//         return Ok(None);
//     }

//     if model_id.len() > 1 {
//         let mut values: ParsedInputMap = value.try_into()?;
//         let mut pairs: Vec<(Field, PrismaValue)> = vec![];

//         for field in model_id.clone() {
//             let val = match values.remove(field.name()) {
//                 Some(v) => v.try_into()?,
//                 None => PrismaValue::Null,
//             };

//             pairs.push((field, val));
//         }

//         Ok(Some(pairs.into()))
//     } else {
//         let value: PrismaValue = value.try_into()?;
//         match value {
//             PrismaValue::Null => Ok(None),
//             value => {
//                 let field = model_id
//                     .fields()
//                     .nth(0)
//                     .map(|f| f.clone())
//                     .expect("Expected model identifier to have at least one field.");

//                 Ok(Some(RecordIdentifier::new(vec![(field, value)])))
//             }
//         }
//     }
// }
