use super::*;
use crate::{
    query_document::*, schema::*, ResultInfo,
};
use connector::Query;

pub struct QueryBuilder {
    pub query_schema: QuerySchemaRef,
}

// Todo:
// - Use error collections instead of letting first error win.
// - UUID ids are not encoded in any useful way in the schema.
// - Alias handling in query names.
impl QueryBuilder {
    pub fn new(query_schema: QuerySchemaRef) -> Self {
        QueryBuilder { query_schema }
    }

    // WIP
    pub fn build_test(self, query_doc: QueryDocument) -> QueryBuilderResult<Vec<(Query, ResultInfo)>> {
        unimplemented!()
    }

    // /// Builds queries from a query document.
    // pub fn build(self, query_doc: QueryDocument) -> CoreResult<Vec<QueryPair>> {
    //     query_doc
    //         .operations
    //         .into_iter()
    //         .map(|op| self.map_operation(op))
    //         .collect::<QueryBuilderResult<Vec<Vec<QueryPair>>>>()
    //         .map(|vec| vec.into_iter().flatten().collect())
    //         .map_err(|err| err.into())
    // }

    // /// Maps an operation to a query.
    // fn map_operation(&self, operation: Operation) -> QueryBuilderResult<Vec<QueryPair>> {
    //     match operation {
    //         Operation::Read(read_op) => self.map_read_operation(read_op),
    //         Operation::Write(write_op) => self.map_write_operation(write_op),
    //     }
    // }

    // /// Maps a read operation to one or more queries.
    // fn map_read_operation(&self, read_op: ReadOperation) -> QueryBuilderResult<Vec<QueryPair>> {
    //     let query_object = self.query_schema.query();
    //     let parsed = self.parse_object(&read_op.selections, &query_object)?;

    //     // Special treatment on read root: all fields map to a model operation.
    //     // This means: Find matching schema field (which is a bit redundant here,
    //     // because it was done during object parsing already).
    //     // Then, for each field on the query object: build a query.
    //     parsed
    //         .fields
    //         .into_iter()
    //         .map(|parsed_field| {
    //             let field = query_object
    //                 .find_field(&parsed_field.name)
    //                 .expect("Expected validation to guarantee existing field on Query object.");

    //             let field_operation = field
    //                 .operation
    //                 .as_ref()
    //                 .expect("Expected Query object fields to always have an associated operation.");

    //             self.map_read(parsed_field, field_operation, &field.field_type)
    //         })
    //         .collect()
    // }

    // fn map_read(
    //     &self,
    //     parsed_field: ParsedField,
    //     operation: &ModelOperation,
    //     field_type: &OutputTypeRef,
    // ) -> QueryBuilderResult<QueryPair> {
    //     let builder = match operation.operation {
    //         OperationTag::FindOne => ReadQueryBuilder::ReadOneRecordBuilder(ReadOneRecordBuilder::new(
    //             parsed_field,
    //             Arc::clone(&operation.model),
    //         )),
    //         OperationTag::FindMany => ReadQueryBuilder::ReadManyRecordsBuilder(ReadManyRecordsBuilder::new(
    //             parsed_field,
    //             Arc::clone(&operation.model),
    //         )),
    //         OperationTag::Aggregate(_) => ReadQueryBuilder::AggregateRecordsBuilder(AggregateRecordsBuilder::new(
    //             parsed_field,
    //             Arc::clone(&operation.model),
    //         )),
    //         _ => unreachable!(),
    //     };

    //     builder
    //         .build()
    //         .map(Query::Read)
    //         .map(|res| (res, ResultResolutionStrategy::Serialize(Arc::clone(field_type))))
    // }

    // /// Maps a write operation to one or more queries.
    // fn map_write_operation(&self, write_op: WriteOperation) -> QueryBuilderResult<Vec<QueryPair>> {
    //     let mutation_object = self.query_schema.mutation();
    //     let parsed = self.parse_object(&write_op.selections, &mutation_object)?;

    //     // Every top-level field on the parsed object corresponds to one write query root.
    //     // Sub-selections of a write operation field are mapped into an accompanying read query.
    //     parsed
    //         .fields
    //         .into_iter()
    //         .map(|parsed_field| {
    //             let field = mutation_object
    //                 .find_field(&parsed_field.name)
    //                 .expect("Expected validation to guarantee existing field on Mutation object.");

    //             let field_operation = field
    //                 .operation
    //                 .as_ref()
    //                 .expect("Expected Mutation object fields to always have an associated operation.");

    //             let (write_query, result_strategy) = match field_operation.operation {
    //                 OperationTag::CreateOne(ref result_strategy) => {
    //                     let result_strategy = self.resolve_result_strategy(
    //                         &parsed_field,
    //                         &field.field_type,
    //                         result_strategy,
    //                         &field_operation.model,
    //                     )?;

    //                     let write_query = WriteQueryBuilder::CreateBuilder(CreateBuilder::new(
    //                         parsed_field,
    //                         Arc::clone(&field_operation.model),
    //                     ))
    //                     .build()?;

    //                     (write_query, result_strategy)
    //                 }

    //                 OperationTag::UpdateOne(ref result_strategy) => {
    //                     let result_strategy = self.resolve_result_strategy(
    //                         &parsed_field,
    //                         &field.field_type,
    //                         result_strategy,
    //                         &field_operation.model,
    //                     )?;

    //                     let write_query = WriteQueryBuilder::UpdateBuilder(UpdateBuilder::new(
    //                         parsed_field,
    //                         Arc::clone(&field_operation.model),
    //                     ))
    //                     .build()?;

    //                     (write_query, result_strategy)
    //                 }

    //                 OperationTag::DeleteOne(ref result_strategy) => {
    //                     let result_strategy = self.resolve_result_strategy(
    //                         &parsed_field,
    //                         &field.field_type,
    //                         result_strategy,
    //                         &field_operation.model,
    //                     )?;

    //                     let write_query = WriteQueryBuilder::DeleteBuilder(DeleteBuilder::new(
    //                         parsed_field,
    //                         Arc::clone(&field_operation.model),
    //                     ))
    //                     .build()?;

    //                     (write_query, result_strategy)
    //                 }

    //                 OperationTag::UpsertOne(ref result_strategy) => {
    //                     let result_strategy = self.resolve_result_strategy(
    //                         &parsed_field,
    //                         &field.field_type,
    //                         result_strategy,
    //                         &field_operation.model,
    //                     )?;

    //                     let write_query = WriteQueryBuilder::UpsertBuilder(UpsertBuilder::new(
    //                         parsed_field,
    //                         Arc::clone(&field_operation.model),
    //                     ))
    //                     .build()?;

    //                     (write_query, result_strategy)
    //                 }

    //                 OperationTag::DeleteMany(ref result_strategy) => {
    //                     let result_strategy = self.resolve_result_strategy(
    //                         &parsed_field,
    //                         &field.field_type,
    //                         result_strategy,
    //                         &field_operation.model,
    //                     )?;

    //                     let write_query = WriteQueryBuilder::DeleteManyBuilder(DeleteManyBuilder::new(
    //                         parsed_field,
    //                         Arc::clone(&field_operation.model),
    //                     ))
    //                     .build()?;

    //                     (write_query, result_strategy)
    //                 }

    //                 OperationTag::UpdateMany(ref result_strategy) => {
    //                     let result_strategy = self.resolve_result_strategy(
    //                         &parsed_field,
    //                         &field.field_type,
    //                         result_strategy,
    //                         &field_operation.model,
    //                     )?;

    //                     let write_query = WriteQueryBuilder::UpdateManyBuilder(UpdateManyBuilder::new(
    //                         parsed_field,
    //                         Arc::clone(&field_operation.model),
    //                     ))
    //                     .build()?;

    //                     (write_query, result_strategy)
    //                 }

    //                 _ => unreachable!(),
    //             };

    //             Ok((Query::Write(write_query), result_strategy))
    //         })
    //         .collect()
    // }

    // fn resolve_result_strategy(
    //     &self,
    //     parsed_field: &ParsedField,
    //     field_type: &OutputTypeRef,
    //     result_tag: &OperationTag,
    //     model: &ModelRef,
    // ) -> QueryBuilderResult<ResultResolutionStrategy> {
    //     Ok(match result_tag.borrow() {
    //         OperationTag::FindOne => {
    //             // Dependent model operation
    //             let model_op = ModelOperation {
    //                 model: Arc::clone(model),
    //                 operation: OperationTag::FindOne,
    //             };

    //             let query = self.derive_read_one_query(&parsed_field, model_op, field_type)?;
    //             ResultResolutionStrategy::Dependent(Box::new(query))
    //         }

    //         OperationTag::CoerceResultToOutputType => ResultResolutionStrategy::Serialize(field_type.clone()),

    //         _ => unreachable!(),
    //     })
    // }

    // fn derive_read_one_query(
    //     &self,
    //     field: &ParsedField,
    //     model_op: ModelOperation,
    //     output_type: &OutputTypeRef,
    // ) -> QueryBuilderResult<QueryPair> {
    //     let derived_field = ParsedField {
    //         name: field.name.clone(),
    //         alias: field.alias.clone(),
    //         arguments: vec![],
    //         sub_selections: field.sub_selections.clone(),
    //     };

    //     self.map_read(derived_field, &model_op, output_type)
    // }


}
