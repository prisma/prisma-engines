use super::{internal::serialize_internal, response::*, *};
use crate::{CoreError, ExpressionResult, OutputFieldRef, OutputType, QueryResult};

use prisma_models::PrismaValue;
use std::borrow::Borrow;

#[derive(Debug)]
pub struct IrSerializer {
    /// Serialization key for root DataItem
    /// Note: This will change
    pub key: String,

    /// Output field describing the possible shape of the result
    pub output_field: OutputFieldRef,
}

impl IrSerializer {
    pub fn serialize(&self, result: ExpressionResult) -> crate::Result<ResponseData> {
        match result {
            ExpressionResult::Query(QueryResult::Json(json)) => {
                Ok(ResponseData::new(self.key.clone(), Item::Json(json)))
            }

            ExpressionResult::Query(r) => {
                let serialized = serialize_internal(r, &self.output_field, false)?;

                // On the top level, each result boils down to a exactly a single serialized result.
                // All checks for lists and optionals have already been performed during the recursion,
                // so we just unpack the only result possible.
                // Todo: The following checks feel out of place. This probably needs to be handled already one level deeper.
                let result = if serialized.is_empty() {
                    if !self.output_field.is_required {
                        Item::Value(PrismaValue::Null(TypeHint::Unknown))
                    } else {
                        match self.output_field.field_type.borrow() {
                            OutputType::List(_) => Item::list(Vec::new()),
                            other => return Err(CoreError::SerializationError(format!(
                                "Invalid response data: the query result was required, but an empty {:?} was returned instead.",
                                other
                            ))),
                        }
                    }
                } else {
                    let (_, item) = serialized.into_iter().take(1).next().unwrap();
                    item
                };

                Ok(ResponseData::new(self.key.clone(), result))
            }

            ExpressionResult::Empty => panic!("Domain logic error: Attempted to serialize empty result."),

            _ => panic!(
                "Domain logic error: Attempted to serialize non-query result {:?}.",
                result
            ),
        }
    }
}
