use crate::{QueryArguments, ScalarListValues};
// use prisma_models::{GraphqlId, ManyRecords, PrismaValue, SingleRecord};
use prisma_models::ManyRecords;

// #[derive(Debug, Default)]
// pub struct ReadQueryResult {
//     /// Orignal query key.
//     pub name: String,

//     /// Designates the key under which the result is serialized.
//     pub alias: Option<String>,

//     /// Holds an ordered list of selected field names for each contained record.
//     pub fields: Vec<String>,

//     /// Scalar field results
//     pub scalars: ManyRecords,

//     /// Nested queries results
//     pub nested: Vec<ReadQueryResult>,

//     /// Scalar list results, field names mapped to their results
//     pub lists: Vec<(String, Vec<ScalarListValues>)>,

//     /// Required for result processing
//     pub query_arguments: QueryArguments,

//     /// Name of the id field of the contained records.
//     pub id_field: String,
// }

#[derive(Debug, Clone)]
pub struct ReadQueryResult {
    /// Orignal query key.
    pub name: String,

    /// Designates the key under which the result is serialized.
    pub alias: Option<String>,

    /// Actual content of the result
    pub content: ResultContent,
}

#[derive(Debug, Clone)]
pub enum ResultContent {
    RecordSelection(RecordSelection),
    Count(usize),
}

#[derive(Debug, Default, Clone)]
pub struct RecordSelection {
    /// Holds an ordered list of selected field names for each contained record.
    pub fields: Vec<String>,

    /// Scalar field results
    pub scalars: ManyRecords,

    /// Nested queries results
    pub nested: Vec<ReadQueryResult>,

    /// Scalar list results, field names mapped to their results
    pub lists: Vec<(String, Vec<ScalarListValues>)>,

    /// Required for result processing
    pub query_arguments: QueryArguments,

    /// Name of the id field of the contained records.
    pub id_field: String,
}
