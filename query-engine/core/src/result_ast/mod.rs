use connector::AggregationRow;
use query_structure::{ManyRecords, Model, RawJson, SelectionResult, VirtualSelection};

#[derive(Debug, Clone)]
pub enum QueryResult {
    Id(Option<SelectionResult>),
    Count(usize),
    RecordSelection(Option<Box<RecordSelection>>),
    RecordSelectionWithRelations(Box<RecordSelectionWithRelations>),
    RawJson(RawJson),
    RecordAggregations(RecordAggregations),
    Unit,
}

#[derive(Debug, Clone)]
pub struct RecordSelectionWithRelations {
    /// Name of the query.
    pub(crate) name: String,

    /// Holds an ordered list of selected field names for each contained record.
    pub(crate) fields: Vec<String>,

    /// Holds the list of virtual selections included in the query result.
    /// TODO: in the future it should be covered by [`RecordSelection::fields`] by storing ordered
    /// `Vec<SelectedField>` or `FieldSelection` instead of `Vec<String>`.
    pub(crate) virtuals: Vec<VirtualSelection>,

    /// Selection results
    pub(crate) records: ManyRecords,

    pub(crate) nested: Vec<RelationRecordSelection>,

    /// The model of the contained records.
    pub(crate) model: Model,
}

impl From<RecordSelectionWithRelations> for QueryResult {
    fn from(value: RecordSelectionWithRelations) -> Self {
        QueryResult::RecordSelectionWithRelations(Box::new(value))
    }
}

#[derive(Debug, Clone)]
pub struct RelationRecordSelection {
    /// Name of the relation.
    pub name: String,
    /// Holds an ordered list of selected field names for each contained record.
    pub fields: Vec<String>,
    /// Holds the list of virtual selections included in the query result.
    pub virtuals: Vec<VirtualSelection>,
    /// The model of the contained records.
    pub model: Model,
    /// Nested relation selections
    pub nested: Vec<RelationRecordSelection>,
}

#[derive(Debug, Clone)]
pub struct RecordSelection {
    /// Name of the query.
    pub(crate) name: String,

    /// Holds an ordered list of selected field names for each contained record.
    pub(crate) fields: Vec<String>,

    /// Selection results (includes scalar and virtual fields)
    pub(crate) records: ManyRecords,

    /// Nested query results
    // Todo this is only here because reads are still resolved in one go
    pub(crate) nested: Vec<QueryResult>,

    /// The model of the contained records.
    pub(crate) model: Model,

    /// The list of virtual selections included in the query result.
    /// TODO: in the future it should be covered by [`RecordSelection::fields`] by storing ordered
    /// `Vec<SelectedField>` or `FieldSelection` instead of `Vec<String>`.
    pub(crate) virtual_fields: Vec<VirtualSelection>,
}

impl From<RecordSelection> for QueryResult {
    fn from(selection: RecordSelection) -> Self {
        QueryResult::RecordSelection(Some(Box::new(selection)))
    }
}

#[derive(Debug, Clone)]
pub struct RecordAggregations {
    /// Ordered list of selected fields as defined by the original incoming query.
    pub(crate) selection_order: Vec<(String, Option<Vec<String>>)>,

    /// Actual aggregation results.
    pub(crate) results: Vec<AggregationRow>,
}
