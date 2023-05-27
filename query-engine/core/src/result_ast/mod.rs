use connector::{AggregationRow, RelAggregationRow};
use prisma_models::{ManyRecords, ModelRef, SelectionResult};

#[derive(Debug, Clone)]
pub(crate) enum QueryResult {
    Id(Option<SelectionResult>),
    Count(usize),
    RecordSelection(Box<RecordSelection>),
    Json(serde_json::Value),
    RecordAggregations(RecordAggregations),
    Unit,
}

// Todo: In theory, much of this info can go into the serializer as soon as the read results are resolved in a flat tree.
#[derive(Debug, Clone)]
pub struct RecordSelection {
    /// Name of the query.
    pub(crate) name: String,

    /// Holds an ordered list of selected field names for each contained record.
    pub(crate) fields: Vec<String>,

    /// Scalar field results
    pub(crate) scalars: ManyRecords,

    /// Nested query results
    // Todo this is only here because reads are still resolved in one go
    pub(crate) nested: Vec<QueryResult>,

    /// The model of the contained records.
    pub(crate) model: ModelRef,

    /// Holds an ordered list of aggregation selections results for each contained record
    pub(crate) aggregation_rows: Option<Vec<RelAggregationRow>>,
}

impl From<RecordSelection> for QueryResult {
    fn from(selection: RecordSelection) -> Self {
        QueryResult::RecordSelection(Box::new(selection))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RecordAggregations {
    /// Ordered list of selected fields as defined by the original incoming query.
    pub(crate) selection_order: Vec<(String, Option<Vec<String>>)>,

    /// Actual aggregation results.
    pub(crate) results: Vec<AggregationRow>,
}
