use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

use crate::result_node::ResultNode;
use query_builder::DbQuery;
use query_core::{DataExpectation, DataRule};
use query_structure::{InternalEnum, PrismaValue, PrismaValueType, ScalarWriteOperation, TaggedPrismaValue};
use serde::Serialize;
use thiserror::Error;

mod format;

#[derive(Debug, Serialize)]
pub struct Binding {
    pub name: Cow<'static, str>,
    pub expr: Expression,
}

impl Binding {
    pub fn new(name: impl Into<Cow<'static, str>>, expr: Expression) -> Self {
        Self {
            name: name.into(),
            expr,
        }
    }
}

impl std::fmt::Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.name, self.expr)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinExpression {
    pub child: Expression,
    pub on: Vec<(String, String)>,
    pub parent_field: String,
    pub is_relation_unique: bool,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "args", rename_all = "camelCase")]
pub enum Expression {
    /// Expression that evaluates to a plain value.
    Value(PrismaValue),

    /// Sequence of statements. The whole sequence evaluates to the result of the last expression.
    Seq(Vec<Expression>),

    /// Get binding value.
    Get { name: Cow<'static, str> },

    /// A lexical scope with let-bindings.
    Let {
        bindings: Vec<Binding>,
        expr: Box<Expression>,
    },

    /// Gets the first non-empty value from a list of bindings.
    GetFirstNonEmpty { names: Vec<Cow<'static, str>> },

    /// A database query that returns data.
    Query(DbQuery),

    /// A database query that returns the number of affected rows.
    Execute(DbQuery),

    /// Reverses the result of an expression in memory.
    Reverse(Box<Expression>),

    /// Sums a list of scalars returned by the expressions.
    Sum(Vec<Expression>),

    /// Concatenates a list of lists.
    Concat(Vec<Expression>),

    /// Asserts that the result of the expression is at most one record.
    Unique(Box<Expression>),

    /// Asserts that the result of the expression is at least one record.
    Required(Box<Expression>),

    /// Application-level join.
    Join {
        parent: Box<Expression>,
        children: Vec<JoinExpression>,
    },

    /// Get a field from a record or records. If the argument is a list of records,
    /// returns a list of values of this field.
    MapField { field: String, records: Box<Expression> },

    /// Run the query inside a transaction
    Transaction(Box<Expression>),

    /// Data mapping
    DataMap {
        expr: Box<Expression>,
        structure: ResultNode,
        enums: EnumsMap,
    },

    /// Validates the expression according to the data rule and throws an error if it doesn't match.
    Validate {
        expr: Box<Expression>,
        rules: Vec<DataRule>,
        error_identifier: &'static str,
        context: serde_json::Value,
    },

    /// Checks if `value` satisifies the `rule`, and executes `then` if it does, or `r#else` if it doesn't.
    If {
        value: Box<Expression>,
        rule: DataRule,
        then: Box<Expression>,
        r#else: Box<Expression>,
    },

    /// Unit value.
    Unit,

    /// Difference between the sets of rows in `from` and `to` (i.e. `from - to`,
    /// or the set of rows that are in `from` but not in `to`).
    Diff { from: Box<Expression>, to: Box<Expression> },

    /// Deduplicates the result of an expression by a list of fields.
    DistinctBy { expr: Box<Expression>, fields: Vec<String> },

    /// Pagination over the result of an expression.
    Paginate {
        expr: Box<Expression>,
        pagination: Pagination,
    },

    /// Initializes a record with a set of initializers.
    InitializeRecord {
        expr: Box<Expression>,
        fields: BTreeMap<String, FieldInitializer>,
    },

    /// Applies a set of operations to fields of a record.
    MapRecord {
        expr: Box<Expression>,
        fields: BTreeMap<String, FieldOperation>,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum FieldInitializer {
    LastInsertId,
    Value(#[serde(serialize_with = "serialize_tagged_value")] PrismaValue),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum FieldOperation {
    Set(#[serde(serialize_with = "serialize_tagged_value")] PrismaValue),
    Add(#[serde(serialize_with = "serialize_tagged_value")] PrismaValue),
    Subtract(#[serde(serialize_with = "serialize_tagged_value")] PrismaValue),
    Multiply(#[serde(serialize_with = "serialize_tagged_value")] PrismaValue),
    Divide(#[serde(serialize_with = "serialize_tagged_value")] PrismaValue),
}

impl TryFrom<ScalarWriteOperation> for FieldOperation {
    type Error = UnsupportedScalarWriteOperation;

    fn try_from(op: ScalarWriteOperation) -> Result<Self, Self::Error> {
        match op {
            ScalarWriteOperation::Set(val) => Ok(Self::Set(val)),
            ScalarWriteOperation::Add(val) => Ok(Self::Add(val)),
            ScalarWriteOperation::Subtract(val) => Ok(Self::Subtract(val)),
            ScalarWriteOperation::Multiply(val) => Ok(Self::Multiply(val)),
            ScalarWriteOperation::Divide(val) => Ok(Self::Divide(val)),
            ScalarWriteOperation::Field(_) | ScalarWriteOperation::Unset(_) => Err(UnsupportedScalarWriteOperation(op)),
        }
    }
}

#[derive(Debug, Error)]
#[error("unsupported scalar write operation: {0:?}")]
pub struct UnsupportedScalarWriteOperation(ScalarWriteOperation);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    cursor: Option<HashMap<String, PrismaValue>>,
    take: Option<i64>,
    skip: Option<i64>,
    linking_fields: Option<Vec<String>>,
}

impl Pagination {
    pub fn new(cursor: Option<HashMap<String, PrismaValue>>, take: Option<i64>, skip: Option<i64>) -> Self {
        Self {
            cursor,
            take,
            skip,
            linking_fields: None,
        }
    }

    pub fn with_linking_fields(self, linking_fields: impl Into<Vec<String>>) -> Self {
        Self {
            linking_fields: Some(linking_fields.into()),
            ..self
        }
    }

    pub fn cursor(&self) -> Option<&HashMap<String, PrismaValue>> {
        self.cursor.as_ref()
    }

    pub fn take(&self) -> Option<i64> {
        self.take
    }

    pub fn skip(&self) -> Option<i64> {
        self.skip
    }
}

#[derive(Debug, Default, Serialize)]
pub struct EnumsMap(BTreeMap<String, BTreeMap<String, String>>);

impl EnumsMap {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, r#enum: InternalEnum) {
        let walker = r#enum.walker();
        if !self.0.contains_key(walker.name()) {
            self.0.insert(
                walker.name().to_owned(),
                walker
                    .values()
                    .map(|v| (v.database_name().to_owned(), v.name().to_owned()))
                    .collect(),
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ExpressionType {
    Scalar,
    Record,
    List(Box<ExpressionType>),
    Dynamic,
    Unit,
}

impl ExpressionType {
    pub fn is_list(&self) -> bool {
        matches!(self, ExpressionType::List(_) | ExpressionType::Dynamic)
    }

    pub fn from_value_type(value_type: PrismaValueType) -> Self {
        match value_type {
            PrismaValueType::Any => ExpressionType::Dynamic,
            PrismaValueType::Array(inner) => ExpressionType::List(Box::new(ExpressionType::from_value_type(*inner))),
            PrismaValueType::Object => ExpressionType::Record,
            _ => ExpressionType::Scalar,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PrettyPrintError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
}

impl Expression {
    pub fn pretty_print(&self, color: bool, width: usize) -> Result<String, PrettyPrintError> {
        let arena = pretty::Arena::new();
        let builder = format::PrettyPrinter::new(&arena);
        let doc = builder.expression(self);

        let mut buf = if color {
            pretty::termcolor::Buffer::ansi()
        } else {
            pretty::termcolor::Buffer::no_color()
        };

        doc.render_colored(width, &mut buf)?;
        Ok(String::from_utf8(buf.into_inner())?)
    }

    pub fn r#type(&self) -> ExpressionType {
        match self {
            Expression::Value(value) => ExpressionType::from_value_type(value.r#type()),
            Expression::Seq(vec) => vec.iter().last().map_or(ExpressionType::Scalar, Expression::r#type),
            Expression::Get { .. } => ExpressionType::Dynamic,
            Expression::Let { expr, .. } => expr.r#type(),
            Expression::GetFirstNonEmpty { .. } => ExpressionType::Dynamic,
            Expression::Query(_) => ExpressionType::List(Box::new(ExpressionType::Record)),
            Expression::Execute(_) => ExpressionType::Scalar,
            Expression::Reverse(expression) => expression.r#type(),
            Expression::Sum(_) => ExpressionType::Scalar,
            Expression::Concat(vec) => ExpressionType::List(Box::new(
                vec.iter().last().map_or(ExpressionType::Scalar, Expression::r#type),
            )),
            Expression::Unique(expression) => match expression.r#type() {
                ExpressionType::List(inner) => inner.as_ref().clone(),
                _ => expression.r#type(),
            },
            Expression::Required(expression) => expression.r#type(),
            Expression::Join { parent, .. } => parent.r#type(),
            Expression::MapField { records, .. } => records.r#type(),
            Expression::Transaction(expression) => expression.r#type(),
            Expression::DataMap { expr, .. } => expr.r#type(),
            Expression::Validate { expr, .. } => expr.r#type(),
            Expression::If { then, r#else, .. } => {
                let then_type = then.r#type();
                let else_type = r#else.r#type();
                if then_type == else_type {
                    then_type
                } else {
                    ExpressionType::Dynamic
                }
            }
            Expression::Unit => ExpressionType::Unit,
            Expression::Diff { from, .. } => from.r#type(),
            Expression::DistinctBy { expr, .. } => expr.r#type(),
            Expression::Paginate { expr, .. } => expr.r#type(),
            Expression::InitializeRecord { expr, .. } => expr.r#type(),
            Expression::MapRecord { expr, .. } => expr.r#type(),
        }
    }

    pub fn validate_expectation(expectation: &DataExpectation, expr: Expression) -> Expression {
        Expression::Validate {
            expr: expr.into(),
            rules: expectation.rules().to_vec(),
            error_identifier: expectation.error().id(),
            context: expectation.error().context(),
        }
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pretty_print(false, 80).map_err(|_| std::fmt::Error)?.fmt(f)
    }
}

fn serialize_tagged_value<S>(obj: &PrismaValue, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    TaggedPrismaValue::from(obj).serialize(serializer)
}
