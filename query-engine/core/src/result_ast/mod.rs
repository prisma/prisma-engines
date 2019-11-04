use connector::{QueryArguments, ScalarListValues};
use prisma_models::{GraphqlId, ManyRecords};

#[derive(Debug, Clone)]
pub enum QueryResult {
    Id(GraphqlId),
    Count(usize),
    RecordSelection(RecordSelection),
    Unit,
}

// Todo: In theory, much of this info can go into the serializer as soon as the read results are resolved in a flat tree.
#[derive(Debug, Default, Clone)]
pub struct RecordSelection {
    /// Name of the query.
    pub name: String,

    /// Holds an ordered list of selected field names for each contained record.
    pub fields: Vec<String>,

    /// Scalar field results
    pub scalars: ManyRecords,

    /// Nested queries results
    // Todo this is only here because reads are still resolved in one go
    pub nested: Vec<QueryResult>,

    /// Scalar list results, field names mapped to their results
    pub lists: Vec<(String, Vec<ScalarListValues>)>,

    /// Required for result processing
    pub query_arguments: QueryArguments,

    /// Name of the id field of the contained records.
    pub id_field: String,
}
