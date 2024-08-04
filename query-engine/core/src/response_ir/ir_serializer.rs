use super::{internal::serialize_internal, response::*, *};
use crate::{CoreError, ExpressionResult, QueryResult};
use query_structure::PrismaValue;
use schema::{OutputField, QuerySchema};

#[derive(Debug)]
pub struct IrSerializer<'a> {
    /// Serialization key for root DataItem
    /// Note: This will change
    pub(crate) key: String,

    /// Output field describing the possible shape of the result
    pub(crate) output_field: OutputField<'a>,
}

impl<'a> IrSerializer<'a> {
    pub(crate) fn serialize(
        &self,
        result: ExpressionResult,
        query_schema: &'a QuerySchema,
    ) -> crate::Result<ResponseData> {
        let _span = info_span!("prisma:engine:serialize", user_facing = true);
        match result {
            ExpressionResult::Query(QueryResult::RawJson(json)) => {
                Ok(ResponseData::new(self.key.clone(), Item::RawJson(json)))
            }

            ExpressionResult::Query(r) => {
                let serialized = serialize_internal(r, &self.output_field, false, query_schema)?;

                // On the top level, each result boils down to a exactly a single serialized result.
                // All checks for lists and optionals have already been performed during the recursion,
                // so we just unpack the only result possible.
                // Todo: The following checks feel out of place. This probably needs to be handled already one level deeper.
                let result = if serialized.is_empty() {
                    if self.output_field.is_nullable {
                        Item::Value(PrismaValue::Null)
                    } else {
                        match self.output_field.field_type() {
                            t if t.is_list() => Item::list(Vec::new()),
                            _ => {
                                return Err(CoreError::SerializationError(format!(
                                    "Query {} is required to return data, but found no record(s).",
                                    self.output_field.name()
                                )))
                            }
                        }
                    }
                } else {
                    let (_, item) = serialized.into_iter().take(1).next().unwrap();
                    item
                };

                Ok(ResponseData::new(self.key.clone(), result))
            }

            ExpressionResult::Empty => panic!("Internal error: Attempted to serialize empty result."),

            _ => panic!("Internal error: Attempted to serialize non-query result {result:?}."),
        }
    }
}
