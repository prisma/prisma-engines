use std::fmt;

use crate::{DataDependencyError, DataOperation, DependentOperation, QueryGraphError, RelationType};
use bon::bon;
use query_structure::{DomainError, Model, Relation, RelationFieldRef};
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

impl From<RelationViolation> for DataDependencyError {
    fn from(error: RelationViolation) -> Self {
        Self::RelationViolation {
            relation: error.relation,
            model_a: error.model_a,
            model_b: error.model_b,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MissingRecord {
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

impl From<MissingRecord> for DataDependencyError {
    fn from(error: MissingRecord) -> Self {
        Self::MissingRecord {
            operation: error.operation,
        }
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
            relation_type: relation.into(),
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

impl From<MissingRelatedRecord> for DataDependencyError {
    fn from(error: MissingRelatedRecord) -> Self {
        Self::MissingRelatedRecord {
            model: error.model,
            relation: error.relation,
            relation_type: error.relation_type,
            operation: error.operation,
            needed_for: error.needed_for,
        }
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

impl From<IncompleteConnectInput> for DataDependencyError {
    fn from(error: IncompleteConnectInput) -> Self {
        Self::IncompleteConnectInput {
            expected_rows: error.expected_rows,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IncompleteConnectOutput {
    expected_rows: usize,
    relation: String,
    relation_type: RelationType,
}

#[bon]
impl IncompleteConnectOutput {
    #[builder]
    pub fn new(expected_rows: usize, relation: &Relation) -> Self {
        Self {
            expected_rows,
            relation: relation.name(),
            relation_type: relation.into(),
        }
    }
}

impl From<IncompleteConnectOutput> for DataDependencyError {
    fn from(error: IncompleteConnectOutput) -> Self {
        Self::IncompleteConnectOutput {
            expected_rows: error.expected_rows,
            relation: error.relation,
            relation_type: error.relation_type,
        }
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

impl From<RecordsNotConnected> for DataDependencyError {
    fn from(error: RecordsNotConnected) -> Self {
        Self::RecordsNotConnected {
            relation: error.relation,
            parent: error.parent,
            child: error.child,
        }
    }
}
