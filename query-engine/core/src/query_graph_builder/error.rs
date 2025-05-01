use std::fmt;

use crate::{DataDependencyError, QueryGraphError};
use query_structure::{DomainError, Model, Relation, RelationFieldRef, SelectionResult};
use typed_builder::TypedBuilder;
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

#[derive(Debug, Clone)]
pub struct RelationViolation {
    pub relation_name: String,
    pub model_a_name: String,
    pub model_b_name: String,
}

impl From<RelationFieldRef> for RelationViolation {
    fn from(rf: RelationFieldRef) -> Self {
        Self::from(&rf)
    }
}

impl From<&RelationFieldRef> for RelationViolation {
    fn from(rf: &RelationFieldRef) -> Self {
        let relation = rf.relation();
        let relation_name = relation.name();
        let [model_a_name, model_b_name] = relation.walker().models().map(|m| rf.dm.walk(m).name().to_owned());

        Self {
            relation_name,
            model_a_name,
            model_b_name,
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

impl fmt::Display for RelationViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            relation_name,
            model_a_name,
            model_b_name,
        } = self;

        write!(f, "The change you are trying to make would violate the required relation '{relation_name}' between the `{model_a_name}` and `{model_b_name}` models.")
    }
}

impl DataDependencyError for RelationViolation {
    fn id(&self) -> &'static str {
        "RELATION_VIOLATION"
    }

    fn to_runtime_error(&self, _results: &[SelectionResult]) -> QueryGraphBuilderError {
        QueryGraphBuilderError::RelationViolation(self.clone())
    }
}

#[derive(Debug, TypedBuilder)]
pub(crate) struct MissingRelatedRecord {
    model: Model,
    relation: Relation,
    operation: DataOperation,
    #[builder(default, setter(strip_option))]
    needed_for: Option<DependentOperation>,
}

impl fmt::Display for MissingRelatedRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            model,
            relation,
            operation,
            needed_for,
        } = &self;

        write!(f, "No '{model}' record", model = model.name())?;

        if let Some(needed_for) = needed_for {
            write!(f, " (needed to {})", needed_for)?;
        }

        write!(f, " was found for {operation} on ")?;

        if relation.is_one_to_one() {
            write!(f, "one-to-one")?;
        } else if relation.is_one_to_many() {
            write!(f, "one-to-many")?;
        } else {
            write!(f, "many-to-many")?;
        }

        write!(f, " relation '{relation}'.", relation = relation.name())?;

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
}

#[derive(Debug, TypedBuilder)]
pub(crate) struct MissingRecord {
    operation: DataOperation,
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
}

#[derive(Debug, TypedBuilder)]
pub(crate) struct IncompleteConnectInput {
    expected: usize,
}

impl DataDependencyError for IncompleteConnectInput {
    fn id(&self) -> &'static str {
        "INCOMPLETE_CONNECT_INPUT"
    }

    fn to_runtime_error(&self, results: &[SelectionResult]) -> QueryGraphBuilderError {
        let Self { expected } = self;
        QueryGraphBuilderError::RecordNotFound(format!(
            "Expected {expected} records to be connected, found only {actual}.",
            actual = results.len()
        ))
    }
}

#[derive(Debug, TypedBuilder)]
pub(crate) struct RecordsNotConnected {
    relation: Relation,
    parent: Model,
    child: Model,
}

impl DataDependencyError for RecordsNotConnected {
    fn id(&self) -> &'static str {
        "RECORDS_NOT_CONNECTED"
    }

    fn to_runtime_error(&self, _results: &[SelectionResult]) -> QueryGraphBuilderError {
        QueryGraphBuilderError::RecordsNotConnected {
            relation_name: self.relation.name(),
            parent_name: self.parent.name().into(),
            child_name: self.child.name().into(),
        }
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
pub(crate) enum DependentOperation {
    NestedUpdate,
    DisconnectRecords,
    FindRecords(Model),
    InlineRelation(Model),
    UpdateInlinedRelation(Model),
    CreateInlinedRelation(Model),
    ConnectOrCreateInlinedRelation(Model),
}

impl fmt::Display for DependentOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NestedUpdate => write!(f, "perform a nested update"),
            Self::DisconnectRecords => write!(f, "disconnect existing child records"),
            Self::FindRecords(model) => write!(f, "find '{}' record(s)", model.name()),
            Self::InlineRelation(model) => write!(f, "inline the relation on '{}' record(s)", model.name()),
            Self::UpdateInlinedRelation(model) => {
                write!(f, "update inlined relation for '{}' record(s)", model.name())
            }
            Self::CreateInlinedRelation(model) => {
                write!(f, "create inlined relation for '{}' record(s)", model.name())
            }
            Self::ConnectOrCreateInlinedRelation(model) => {
                write!(f, "create or connect inlined relation for '{}' record(s)", model.name())
            }
        }
    }
}
