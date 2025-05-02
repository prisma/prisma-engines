use std::fmt;

use crate::{DataDependencyError, QueryGraphError};
use bon::bon;
use query_structure::{DomainError, Model, Relation, RelationFieldRef, SelectionResult};
use serde::Serialize;
use user_facing_errors::query_engine::validation::ValidationError;

#[derive(Debug)]
pub enum QueryGraphBuilderError {
    /// Logic error in the construction of the schema.
    /// Not a user error.
    SchemaError(String),

    /// User input error that was't (and can't) be caught
    /// by the general validation during query document parsing.
    InputError(String),

    /// More specific input error for when an argument is missing for a field on a specific model.
    MissingRequiredArgument {
        argument_name: String,
        field_name: String,
        object_name: String,
    },

    /// Wraps the initial parsing stage errors.
    QueryParserError(ValidationError),

    /// Wraps transformation errors from the prisma models.
    DomainError(DomainError),

    /// General assertion error.
    AssertionError(String),

    RelationViolation(RelationViolation),

    RecordsNotConnected {
        relation_name: String,
        parent_name: String,
        // parent_where: Option<Box<RecordFinderInfo>>,
        child_name: String,
        // child_where: Option<Box<RecordFinderInfo>>,
    },

    RecordNotFound(String),

    QueryGraphError(QueryGraphError),
}

impl std::fmt::Display for QueryGraphBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for QueryGraphBuilderError {}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationViolation {
    pub relation: String,
    pub model_a: String,
    pub model_b: String,
}

impl From<RelationFieldRef> for RelationViolation {
    fn from(rf: RelationFieldRef) -> Self {
        Self::from(&rf)
    }
}

impl From<&RelationFieldRef> for RelationViolation {
    fn from(rf: &RelationFieldRef) -> Self {
        let relation = rf.relation();
        let [model_a_name, model_b_name] = relation.walker().models().map(|m| rf.dm.walk(m).name().to_owned());

        Self {
            relation: relation.name(),
            model_a: model_a_name,
            model_b: model_b_name,
        }
    }
}

impl From<DomainError> for QueryGraphBuilderError {
    fn from(err: DomainError) -> Self {
        QueryGraphBuilderError::DomainError(err)
    }
}

impl From<ValidationError> for QueryGraphBuilderError {
    fn from(err: ValidationError) -> Self {
        QueryGraphBuilderError::QueryParserError(err)
    }
}

impl From<QueryGraphError> for QueryGraphBuilderError {
    fn from(err: QueryGraphError) -> Self {
        QueryGraphBuilderError::QueryGraphError(err)
    }
}

impl DataDependencyError for RelationViolation {
    fn id(&self) -> &'static str {
        "RELATION_VIOLATION"
    }

    fn to_runtime_error(&self, _results: &[SelectionResult]) -> QueryGraphBuilderError {
        QueryGraphBuilderError::RelationViolation(self.clone())
    }

    fn context(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct MissingRecord {
    operation: DataOperation,
}

#[bon]
impl MissingRecord {
    #[builder]
    pub fn new(operation: DataOperation) -> Self {
        Self { operation }
    }
}

impl fmt::Display for MissingRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { operation } = self;
        write!(f, "No record was found for {operation}.")
    }
}

impl DataDependencyError for MissingRecord {
    fn id(&self) -> &'static str {
        "MISSING_RECORD"
    }

    fn to_runtime_error(&self, _results: &[SelectionResult]) -> QueryGraphBuilderError {
        QueryGraphBuilderError::RecordNotFound(self.to_string())
    }

    fn context(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissingRelatedRecord {
    model: String,
    relation: String,
    relation_type: RelationType,
    operation: DataOperation,
    needed_for: Option<DependentOperation>,
}

#[bon]
impl MissingRelatedRecord {
    #[builder]
    pub fn new(
        model: &Model,
        relation: &Relation,
        operation: DataOperation,
        needed_for: Option<DependentOperation>,
    ) -> Self {
        Self {
            model: model.name().to_owned(),
            relation: relation.name(),
            relation_type: if relation.is_one_to_one() {
                RelationType::OneToOne
            } else if relation.is_one_to_many() {
                RelationType::OneToMany
            } else {
                RelationType::ManyToMany
            },
            operation,
            needed_for,
        }
    }
}

impl fmt::Display for MissingRelatedRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            model,
            relation,
            relation_type,
            operation,
            needed_for,
        } = &self;

        write!(f, "No '{model}' record")?;
        if let Some(needed_for) = needed_for {
            write!(f, " (needed to {needed_for})")?;
        }
        write!(
            f,
            " was found for {operation} on {relation_type} relation '{relation}'."
        )?;

        Ok(())
    }
}

impl DataDependencyError for MissingRelatedRecord {
    fn id(&self) -> &'static str {
        "MISSING_RELATED_RECORD"
    }

    fn to_runtime_error(&self, _results: &[SelectionResult]) -> QueryGraphBuilderError {
        QueryGraphBuilderError::RecordNotFound(self.to_string())
    }

    fn context(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IncompleteConnectInput {
    expected_rows: usize,
}

#[bon]
impl IncompleteConnectInput {
    #[builder]
    pub fn new(expected_rows: usize) -> Self {
        Self { expected_rows }
    }
}

impl DataDependencyError for IncompleteConnectInput {
    fn id(&self) -> &'static str {
        "INCOMPLETE_CONNECT_INPUT"
    }

    fn to_runtime_error(&self, results: &[SelectionResult]) -> QueryGraphBuilderError {
        let Self { expected_rows } = self;
        QueryGraphBuilderError::RecordNotFound(format!(
            "Expected {expected_rows} records to be connected, found only {actual}.",
            actual = results.len()
        ))
    }

    fn context(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct RecordsNotConnected {
    relation: String,
    parent: String,
    child: String,
}

#[bon]
impl RecordsNotConnected {
    #[builder]
    pub fn new(relation: Relation, parent: Model, child: Model) -> Self {
        Self {
            relation: relation.name(),
            parent: parent.name().into(),
            child: child.name().into(),
        }
    }
}

impl DataDependencyError for RecordsNotConnected {
    fn id(&self) -> &'static str {
        "RECORDS_NOT_CONNECTED"
    }

    fn to_runtime_error(&self, _results: &[SelectionResult]) -> QueryGraphBuilderError {
        QueryGraphBuilderError::RecordsNotConnected {
            relation_name: self.relation.clone(),
            parent_name: self.parent.clone(),
            child_name: self.child.clone(),
        }
    }

    fn context(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(into = "String")]
pub(crate) enum DataOperation {
    Query,
    Update,
    Upsert,
    Delete,
    Disconnect,
    NestedCreate,
    NestedUpdate,
    NestedUpsert,
    NestedDelete,
    NestedSet,
    NestedConnect,
    NestedConnectOrCreate,
}

impl fmt::Display for DataOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::Query => "a query",
            Self::Update => "an update",
            Self::Upsert => "an upsert",
            Self::Delete => "a delete",
            Self::Disconnect => "a disconnect",
            Self::NestedCreate => "a nested create",
            Self::NestedUpdate => "a nested update",
            Self::NestedUpsert => "a nested upsert",
            Self::NestedDelete => "a nested delete",
            Self::NestedSet => "a nested set",
            Self::NestedConnect => "a nested connect",
            Self::NestedConnectOrCreate => "a nested connect or create",
        };
        write!(f, "{str}")
    }
}

impl From<DataOperation> for String {
    fn from(operation: DataOperation) -> Self {
        operation.to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(into = "String")]
pub(crate) enum DependentOperation {
    NestedUpdate,
    DisconnectRecords,
    FindRecords { model: String },
    InlineRelation { model: String },
    UpdateInlinedRelation { model: String },
    CreateInlinedRelation { model: String },
    ConnectOrCreateInlinedRelation { model: String },
}

impl DependentOperation {
    pub fn nested_update() -> Self {
        Self::NestedUpdate
    }

    pub fn disconnect_records() -> Self {
        Self::DisconnectRecords
    }

    pub fn find_records(model: &Model) -> Self {
        Self::FindRecords {
            model: model.name().to_owned(),
        }
    }

    pub fn inline_relation(model: &Model) -> Self {
        Self::InlineRelation {
            model: model.name().to_owned(),
        }
    }

    pub fn update_inlined_relation(model: &Model) -> Self {
        Self::UpdateInlinedRelation {
            model: model.name().to_owned(),
        }
    }

    pub fn create_inlined_relation(model: &Model) -> Self {
        Self::CreateInlinedRelation {
            model: model.name().to_owned(),
        }
    }

    pub fn connect_or_create_inlined_relation(model: &Model) -> Self {
        Self::ConnectOrCreateInlinedRelation {
            model: model.name().to_owned(),
        }
    }
}

impl fmt::Display for DependentOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NestedUpdate => write!(f, "perform a nested update"),
            Self::DisconnectRecords => write!(f, "disconnect existing child records"),
            Self::FindRecords { model } => write!(f, "find '{model}' record(s)"),
            Self::InlineRelation { model } => write!(f, "inline the relation on '{model}' record(s)"),
            Self::UpdateInlinedRelation { model } => {
                write!(f, "update inlined relation for '{model}' record(s)")
            }
            Self::CreateInlinedRelation { model } => {
                write!(f, "create inlined relation for '{model}' record(s)")
            }
            Self::ConnectOrCreateInlinedRelation { model } => {
                write!(f, "create or connect inlined relation for '{model}' record(s)")
            }
        }
    }
}

impl From<DependentOperation> for String {
    fn from(operation: DependentOperation) -> Self {
        operation.to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(into = "String")]
enum RelationType {
    OneToOne,
    OneToMany,
    ManyToMany,
}

impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationType::OneToOne => write!(f, "one-to-one"),
            RelationType::OneToMany => write!(f, "one-to-many"),
            RelationType::ManyToMany => write!(f, "many-to-many"),
        }
    }
}

impl From<RelationType> for String {
    fn from(relation_type: RelationType) -> Self {
        relation_type.to_string()
    }
}
