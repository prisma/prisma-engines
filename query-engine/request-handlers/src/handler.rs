use super::GQLResponse;
use crate::{GQLError, PrismaResponse, RequestBody};
use bigdecimal::BigDecimal;
use futures::FutureExt;
use indexmap::IndexMap;
use quaint::connector::tiberius::Uuid;
use query_core::{
    constants::custom_types,
    protocol::EngineProtocol,
    response_ir::{Item, ResponseData},
    schema::QuerySchemaRef,
    ArgumentValue, ArgumentValueObject, BatchDocument, BatchDocumentTransaction, CompactedDocument, Operation,
    QueryDocument, QueryExecutor, TxId,
};
use query_structure::{parse_datetime, stringify_datetime, PrismaValue};
use std::{collections::HashMap, fmt, panic::AssertUnwindSafe, str::FromStr};

type ArgsToResult = (HashMap<String, ArgumentValue>, IndexMap<String, Item>);

pub struct RequestHandler<'a> {
    executor: &'a (dyn QueryExecutor + Send + Sync + 'a),
    query_schema: &'a QuerySchemaRef,
    engine_protocol: EngineProtocol,
}

impl<'a> fmt::Debug for RequestHandler<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestHandler").finish()
    }
}

impl<'a> RequestHandler<'a> {
    pub fn new(
        executor: &'a (dyn QueryExecutor + Send + Sync + 'a),
        query_schema: &'a QuerySchemaRef,
        engine_protocol: EngineProtocol,
    ) -> Self {
        Self {
            executor,
            query_schema,
            engine_protocol,
        }
    }

    pub async fn handle(&self, body: RequestBody, tx_id: Option<TxId>, trace_id: Option<String>) -> PrismaResponse {
        tracing::debug!("Incoming GraphQL query: {:?}", &body);

        match body.into_doc(self.query_schema) {
            Ok(QueryDocument::Single(query)) => self.handle_single(query, tx_id, trace_id).await,
            Ok(QueryDocument::Multi(batch)) => match batch.compact(self.query_schema) {
                BatchDocument::Multi(batch, transaction) => {
                    self.handle_batch(batch, transaction, tx_id, trace_id).await
                }
                BatchDocument::Compact(compacted) => self.handle_compacted(compacted, tx_id, trace_id).await,
            },

            Err(err) => PrismaResponse::Single(GQLError::from_handler_error(err).into()),
        }
    }

    async fn handle_single(&self, query: Operation, tx_id: Option<TxId>, trace_id: Option<String>) -> PrismaResponse {
        let gql_response = match AssertUnwindSafe(self.handle_request(query, tx_id, trace_id))
            .catch_unwind()
            .await
        {
            Ok(Ok(response)) => response.into(),
            Ok(Err(err)) => GQLError::from_core_error(err).into(),
            Err(err) => GQLError::from_panic_payload(err).into(),
        };

        PrismaResponse::Single(gql_response)
    }

    async fn handle_batch(
        &self,
        queries: Vec<Operation>,
        transaction: Option<BatchDocumentTransaction>,
        tx_id: Option<TxId>,
        trace_id: Option<String>,
    ) -> PrismaResponse {
        match AssertUnwindSafe(self.executor.execute_all(
            tx_id,
            queries,
            transaction,
            self.query_schema.clone(),
            trace_id,
            self.engine_protocol,
        ))
        .catch_unwind()
        .await
        {
            Ok(Ok(responses)) => {
                let gql_responses: Vec<GQLResponse> = responses
                    .into_iter()
                    .map(|response| match response {
                        Ok(data) => data.into(),
                        Err(err) => GQLError::from_core_error(err).into(),
                    })
                    .collect();

                PrismaResponse::Multi(gql_responses.into())
            }
            Ok(Err(err)) => PrismaResponse::Multi(GQLError::from_core_error(err).into()),
            Err(err) => PrismaResponse::Multi(GQLError::from_panic_payload(err).into()),
        }
    }

    async fn handle_compacted(
        &self,
        document: CompactedDocument,
        tx_id: Option<TxId>,
        trace_id: Option<String>,
    ) -> PrismaResponse {
        let plural_name = document.plural_name();
        let singular_name = document.single_name();
        let throw_on_empty = document.throw_on_empty();
        let keys: Vec<String> = document.keys;
        let arguments = document.arguments;
        let nested_selection = document.nested_selection;

        match AssertUnwindSafe(self.handle_request(document.operation, tx_id, trace_id))
            .catch_unwind()
            .await
        {
            Ok(Ok(response_data)) => {
                let mut gql_response: GQLResponse = response_data.into();

                // At this point, many findUnique queries were converted to a single findMany query and that query was run.
                // This means we have a list of results and we need to map each result back to their original findUnique query.
                // `args_to_results` is the data-structure that allows us to do that mapping.
                // It takes the findMany response and converts it to a map of arguments to result.
                // Let's take an example. Given the following batched queries:
                // [
                //    findUnique(where: { id: 1, name: "Bob" }) { id name age },
                //    findUnique(where: { id: 2, name: "Alice" }) { id name age }
                // ]
                // 1. This gets converted to: findMany(where: { OR: [{ id: 1, name: "Bob" }, { id: 2, name: "Alice" }] }) { id name age }
                // 2. Say we get the following result back: [{ id: 1, name: "Bob", age: 18 }, { id: 2, name: "Alice", age: 27 }]
                // 3. We know the inputted arguments are ["id", "name"]
                // 4. So we go over the result and build the following list:
                // [
                //  ({ id: 1, name: "Bob" },   { id: 1, name: "Bob", age: 18 }),
                //  ({ id: 2, name: "Alice" }, { id: 2, name: "Alice", age: 27 })
                // ]
                // 5. Now, given the original findUnique queries and that list, we can easily find back which arguments maps to which result
                // [
                //    findUnique(where: { id: 1, name: "Bob" }) { id name age } -> { id: 1, name: "Bob", age: 18 }
                //    findUnique(where: { id: 2, name: "Alice" }) { id name age } -> { id: 2, name: "Alice", age: 27 }
                // ]
                let args_to_results: Vec<ArgsToResult> = gql_response
                    .take_data(plural_name)
                    .unwrap()
                    .into_list()
                    .unwrap()
                    .index_by(keys.as_slice());

                let results: Vec<GQLResponse> = arguments
                    .into_iter()
                    .map(|args| {
                        let mut responses = GQLResponse::with_capacity(1);
                        // This is step 5 of the comment above.
                        // Copying here is mandatory due to some of the queries
                        // might be repeated with the same arguments in the original
                        // batch. We need to give the same answer for both of them.
                        match Self::find_original_result_from_args(&args_to_results, &args) {
                            Some(result) => {
                                // Filter out all the keys not selected in the
                                // original query.
                                let result: IndexMap<String, Item> = result
                                    .clone()
                                    .into_iter()
                                    .filter(|(k, _)| nested_selection.contains(k))
                                    .collect();

                                responses.insert_data(&singular_name, Item::Map(result));
                            }
                            None if throw_on_empty => responses.insert_error(GQLError::from_user_facing_error(
                                user_facing_errors::query_engine::RecordRequiredButNotFound {
                                    cause: "Expected a record, found none.".to_owned(),
                                }
                                .into(),
                            )),
                            None => responses.insert_data(&singular_name, Item::null()),
                        }

                        responses
                    })
                    .collect();

                PrismaResponse::Multi(results.into())
            }

            Ok(Err(err)) => PrismaResponse::Multi(GQLError::from_core_error(err).into()),

            // panicked
            Err(err) => PrismaResponse::Multi(GQLError::from_panic_payload(err).into()),
        }
    }

    async fn handle_request(
        &self,
        query_doc: Operation,
        tx_id: Option<TxId>,
        trace_id: Option<String>,
    ) -> query_core::Result<ResponseData> {
        self.executor
            .execute(
                tx_id,
                query_doc,
                self.query_schema.clone(),
                trace_id,
                self.engine_protocol,
            )
            .await
    }

    fn find_original_result_from_args<'b>(
        args_to_results: &'b [ArgsToResult],
        input_args: &'b HashMap<String, ArgumentValue>,
    ) -> Option<&'b IndexMap<String, Item>> {
        args_to_results
            .iter()
            .find(|(arg_from_result, _)| Self::compare_args(arg_from_result, input_args))
            .map(|(_, result)| result)
    }

    fn compare_args(left: &HashMap<String, ArgumentValue>, right: &HashMap<String, ArgumentValue>) -> bool {
        left.iter().all(|(key, left_value)| {
            right
                .get(key)
                .map_or(false, |right_value| Self::compare_values(left_value, right_value))
        })
    }

    /// Compares two PrismaValues with special comparisons rules needed because user-inputted values are coerced differently than response values.
    /// We need this when comparing user-inputted values with query response values in the context of compacted queries.
    /// Here are the cases covered:
    /// - DateTime/String: User-input: DateTime / Response: String
    /// - Int/BigInt: User-input: Int / Response: BigInt
    /// - (JSON protocol only) Custom types (eg: { "$type": "BigInt", value: "1" }): User-input: Scalar / Response: Object
    /// - (JSON protocol only) String/Enum: User-input: String / Response: Enum
    /// This should likely _not_ be used outside of this specific context.
    fn compare_values(left: &ArgumentValue, right: &ArgumentValue) -> bool {
        match (left, right) {
            (ArgumentValue::Scalar(PrismaValue::String(t1)), ArgumentValue::Scalar(PrismaValue::DateTime(t2)))
            | (ArgumentValue::Scalar(PrismaValue::DateTime(t2)), ArgumentValue::Scalar(PrismaValue::String(t1))) => {
                parse_datetime(t1)
                    .map(|t1| &t1 == t2)
                    .unwrap_or_else(|_| t1 == stringify_datetime(t2).as_str())
            }
            (ArgumentValue::Scalar(PrismaValue::Int(i1)), ArgumentValue::Scalar(PrismaValue::BigInt(i2)))
            | (ArgumentValue::Scalar(PrismaValue::BigInt(i2)), ArgumentValue::Scalar(PrismaValue::Int(i1))) => {
                *i1 == *i2
            }
            (ArgumentValue::Scalar(PrismaValue::Enum(s1)), ArgumentValue::Scalar(PrismaValue::String(s2)))
            | (ArgumentValue::Scalar(PrismaValue::String(s1)), ArgumentValue::Scalar(PrismaValue::Enum(s2))) => {
                *s1 == *s2
            }
            (ArgumentValue::Object(t1), t2) | (t2, ArgumentValue::Object(t1)) => match Self::unwrap_value(t1) {
                Some(t1) => Self::compare_values(t1, t2),
                None => left == right,
            },
            (ArgumentValue::Scalar(PrismaValue::Float(s1)), ArgumentValue::Scalar(PrismaValue::String(s2)))
            | (ArgumentValue::Scalar(PrismaValue::String(s2)), ArgumentValue::Scalar(PrismaValue::Float(s1))) => {
                BigDecimal::from_str(s2).map(|s2| s2 == *s1).unwrap_or(false)
            }
            (ArgumentValue::Scalar(PrismaValue::String(s1)), ArgumentValue::Scalar(PrismaValue::String(s2))) => {
                match (Uuid::from_str(s1).map(|s1| s1), Uuid::from_str(s2).map(|s2| s2)) {
                    (Ok(s1), Ok(s2)) => s1 == s2,
                    _ => left == right,
                }
            }
            (left, right) => left == right,
        }
    }

    fn unwrap_value(obj: &ArgumentValueObject) -> Option<&ArgumentValue> {
        if obj.len() != 2 {
            return None;
        }

        if !obj.contains_key(custom_types::TYPE) || !obj.contains_key(custom_types::VALUE) {
            return None;
        }

        obj.get(custom_types::VALUE)
    }
}
