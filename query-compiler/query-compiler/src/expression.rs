use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

use crate::{data_mapper::FieldType, result_node::ResultNode};
use bon::{Builder, bon};
use query_builder::DbQuery;
use query_core::{DataExpectation, DataRule};
use query_structure::{InternalEnum, PrismaValue, PrismaValueType, ScalarWriteOperation};
use serde::{Serialize, Serializer, ser::SerializeMap, ser::SerializeTuple};
use serde_json::Value;
use thiserror::Error;

mod format;

#[derive(Debug)]
pub struct Binding {
    pub name: Cow<'static, str>,
    pub expr: Expression,
}

impl Serialize for Binding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.name)?;
        tuple.serialize_element(&self.expr)?;
        tuple.end()
    }
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

#[derive(Debug)]
pub struct JoinExpression {
    pub child: Expression,
    pub on: Vec<(String, String)>,
    pub parent_field: String,
    pub is_relation_unique: bool,
}

impl Serialize for JoinExpression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(4)?;
        tuple.serialize_element(&self.child)?;
        tuple.serialize_element(&self.on)?;
        tuple.serialize_element(&self.parent_field)?;
        tuple.serialize_element(&self.is_relation_unique)?;
        tuple.end()
    }
}

#[derive(Debug)]
pub enum RawResultFieldName {
    Field(String),
    Path(Vec<String>),
}

impl Serialize for RawResultFieldName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Field(name) => name.serialize(serializer),
            Self::Path(path) => path.serialize(serializer),
        }
    }
}

#[derive(Debug)]
pub struct RawResultColumnMapping {
    pub field_name: RawResultFieldName,
    pub column: RawResultColumnRef,
    pub field_type: Option<FieldType>,
}

impl Serialize for RawResultColumnMapping {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(if self.field_type.is_some() { 3 } else { 2 })?;
        tuple.serialize_element(&self.field_name)?;
        tuple.serialize_element(&self.column)?;
        if let Some(field_type) = &self.field_type {
            tuple.serialize_element(field_type)?;
        }
        tuple.end()
    }
}

#[derive(Debug)]
pub enum RawResultColumnRef {
    Index(usize),
    Name(String),
}

impl Serialize for RawResultColumnRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Index(index) => index.serialize(serializer),
            Self::Name(name) => name.serialize(serializer),
        }
    }
}

#[derive(Debug)]
pub struct RawNestedReadQuery {
    pub query: DbQuery,
    pub fields: Vec<RawResultColumnMapping>,
    pub relations: Vec<RawNestedReadRelation>,
}

impl Serialize for RawNestedReadQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(if self.relations.is_empty() { 2 } else { 3 })?;
        tuple.serialize_element(&self.query)?;
        tuple.serialize_element(&self.fields)?;
        if !self.relations.is_empty() {
            tuple.serialize_element(&self.relations)?;
        }
        tuple.end()
    }
}

#[derive(Debug)]
pub enum RawNestedReadRelation {
    Direct(RawNestedReadDirectRelation),
}

impl Serialize for RawNestedReadRelation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Direct(relation) => relation.serialize(serializer),
        }
    }
}

#[derive(Debug)]
pub struct RawNestedReadDirectRelation {
    pub field_name: String,
    pub child: RawNestedReadQuery,
    pub parent_column: RawResultColumnRef,
    pub child_column: RawResultColumnRef,
    pub scope_name: Cow<'static, str>,
    pub is_relation_unique: bool,
    pub operations: InMemoryOps,
}

impl Serialize for RawNestedReadDirectRelation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(8)?;
        tuple.serialize_element("r")?;
        tuple.serialize_element(&self.field_name)?;
        tuple.serialize_element(&self.child)?;
        tuple.serialize_element(&self.parent_column)?;
        tuple.serialize_element(&self.child_column)?;
        tuple.serialize_element(&self.scope_name)?;
        tuple.serialize_element(&self.is_relation_unique)?;
        tuple.serialize_element(&self.operations)?;
        tuple.end()
    }
}

#[derive(Debug)]
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
        can_assume_strict_equality: bool,
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

    /// Raw result-set nested read specialized for query-mode relation loading.
    RawNestedRead {
        query: RawNestedReadQuery,
        unique: bool,
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
    Diff {
        from: Box<Expression>,
        to: Box<Expression>,
        fields: Vec<String>,
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

    /// Process records in memory.
    Process {
        expr: Box<Expression>,
        operations: InMemoryOps,
    },
}

impl Serialize for Expression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Value(value) => serialize_unary("v", value, serializer),
            Self::Seq(expressions) => serialize_unary("s", expressions, serializer),
            Self::Get { name } => serialize_unary("g", name, serializer),
            Self::Let { bindings, expr } => {
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element("l")?;
                tuple.serialize_element(bindings)?;
                tuple.serialize_element(expr)?;
                tuple.end()
            }
            Self::GetFirstNonEmpty { names } => serialize_unary("e", names, serializer),
            Self::Query(query) => serialize_unary("q", query, serializer),
            Self::Execute(query) => serialize_unary("x", query, serializer),
            Self::Sum(expressions) => serialize_unary("+", expressions, serializer),
            Self::Concat(expressions) => serialize_unary("c", expressions, serializer),
            Self::Unique(expr) => serialize_unary("u", expr, serializer),
            Self::Required(expr) => serialize_unary("r", expr, serializer),
            Self::Join {
                parent,
                children,
                can_assume_strict_equality,
            } => {
                let mut tuple = serializer.serialize_tuple(4)?;
                tuple.serialize_element("j")?;
                tuple.serialize_element(parent)?;
                tuple.serialize_element(children)?;
                tuple.serialize_element(can_assume_strict_equality)?;
                tuple.end()
            }
            Self::MapField { field, records } => {
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element("m")?;
                tuple.serialize_element(field)?;
                tuple.serialize_element(records)?;
                tuple.end()
            }
            Self::Transaction(expr) => serialize_unary("t", expr, serializer),
            Self::DataMap { expr, structure, enums } => {
                let mut tuple = serializer.serialize_tuple(if enums.is_empty() { 3 } else { 4 })?;
                tuple.serialize_element("d")?;
                tuple.serialize_element(expr)?;
                tuple.serialize_element(structure)?;
                if !enums.is_empty() {
                    tuple.serialize_element(enums)?;
                }
                tuple.end()
            }
            Self::RawNestedRead { query, unique, enums } => {
                let mut tuple = serializer.serialize_tuple(if enums.is_empty() { 3 } else { 4 })?;
                tuple.serialize_element("n")?;
                tuple.serialize_element(query)?;
                tuple.serialize_element(unique)?;
                if !enums.is_empty() {
                    tuple.serialize_element(enums)?;
                }
                tuple.end()
            }
            Self::Validate {
                expr,
                rules,
                error_identifier,
                context,
            } => {
                let mut tuple = serializer.serialize_tuple(5)?;
                tuple.serialize_element("V")?;
                tuple.serialize_element(expr)?;
                tuple.serialize_element(rules)?;
                tuple.serialize_element(compact_validation_error_identifier(error_identifier))?;
                tuple.serialize_element(&CompactValidationContext {
                    error_identifier,
                    context,
                })?;
                tuple.end()
            }
            Self::If {
                value,
                rule,
                then,
                r#else,
            } => {
                let mut tuple = serializer.serialize_tuple(5)?;
                tuple.serialize_element("?")?;
                tuple.serialize_element(value)?;
                tuple.serialize_element(rule)?;
                tuple.serialize_element(then)?;
                tuple.serialize_element(r#else)?;
                tuple.end()
            }
            Self::Unit => {
                let mut tuple = serializer.serialize_tuple(1)?;
                tuple.serialize_element("0")?;
                tuple.end()
            }
            Self::Diff { from, to, fields } => {
                let mut tuple = serializer.serialize_tuple(4)?;
                tuple.serialize_element("-")?;
                tuple.serialize_element(from)?;
                tuple.serialize_element(to)?;
                tuple.serialize_element(fields)?;
                tuple.end()
            }
            Self::InitializeRecord { expr, fields } => {
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element("i")?;
                tuple.serialize_element(expr)?;
                tuple.serialize_element(fields)?;
                tuple.end()
            }
            Self::MapRecord { expr, fields } => {
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element("M")?;
                tuple.serialize_element(expr)?;
                tuple.serialize_element(fields)?;
                tuple.end()
            }
            Self::Process { expr, operations } => {
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element("p")?;
                tuple.serialize_element(expr)?;
                tuple.serialize_element(operations)?;
                tuple.end()
            }
        }
    }
}

fn serialize_unary<S, T>(tag: &'static str, value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let mut tuple = serializer.serialize_tuple(2)?;
    tuple.serialize_element(tag)?;
    tuple.serialize_element(value)?;
    tuple.end()
}

fn compact_validation_error_identifier(error_identifier: &'static str) -> &'static str {
    match error_identifier {
        "RELATION_VIOLATION" => "r",
        "MISSING_RELATED_RECORD" => "m",
        "MISSING_RECORD" => "M",
        "INCOMPLETE_CONNECT_INPUT" => "i",
        "INCOMPLETE_CONNECT_OUTPUT" => "o",
        "RECORDS_NOT_CONNECTED" => "n",
        _ => error_identifier,
    }
}

struct CompactValidationContext<'a> {
    error_identifier: &'static str,
    context: &'a Value,
}

impl Serialize for CompactValidationContext<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let Value::Object(context) = self.context else {
            return self.context.serialize(serializer);
        };

        match self.error_identifier {
            "RELATION_VIOLATION" => {
                if let (Some(relation), Some(model_a), Some(model_b)) =
                    (context.get("relation"), context.get("modelA"), context.get("modelB"))
                {
                    let mut tuple = serializer.serialize_tuple(3)?;
                    tuple.serialize_element(relation)?;
                    tuple.serialize_element(model_a)?;
                    tuple.serialize_element(model_b)?;
                    return tuple.end();
                }
            }
            "MISSING_RELATED_RECORD" => {
                if let (Some(model), Some(relation), Some(relation_type), Some(operation)) = (
                    context.get("model"),
                    context.get("relation"),
                    context.get("relationType"),
                    context.get("operation"),
                ) {
                    if let Some(needed_for) = context.get("neededFor") {
                        let mut tuple = serializer.serialize_tuple(5)?;
                        tuple.serialize_element(model)?;
                        tuple.serialize_element(relation)?;
                        tuple.serialize_element(relation_type)?;
                        tuple.serialize_element(operation)?;
                        tuple.serialize_element(needed_for)?;
                        return tuple.end();
                    }

                    let mut tuple = serializer.serialize_tuple(4)?;
                    tuple.serialize_element(model)?;
                    tuple.serialize_element(relation)?;
                    tuple.serialize_element(relation_type)?;
                    tuple.serialize_element(operation)?;
                    return tuple.end();
                }
            }
            "MISSING_RECORD" => {
                if let Some(operation) = context.get("operation") {
                    return operation.serialize(serializer);
                }
            }
            "INCOMPLETE_CONNECT_INPUT" => {
                if let Some(expected_rows) = context.get("expectedRows") {
                    return expected_rows.serialize(serializer);
                }
            }
            "INCOMPLETE_CONNECT_OUTPUT" => {
                if let (Some(expected_rows), Some(relation), Some(relation_type)) = (
                    context.get("expectedRows"),
                    context.get("relation"),
                    context.get("relationType"),
                ) {
                    let mut tuple = serializer.serialize_tuple(3)?;
                    tuple.serialize_element(expected_rows)?;
                    tuple.serialize_element(relation)?;
                    tuple.serialize_element(relation_type)?;
                    return tuple.end();
                }
            }
            "RECORDS_NOT_CONNECTED" => {
                if let (Some(relation), Some(parent), Some(child)) =
                    (context.get("relation"), context.get("parent"), context.get("child"))
                {
                    let mut tuple = serializer.serialize_tuple(3)?;
                    tuple.serialize_element(relation)?;
                    tuple.serialize_element(parent)?;
                    tuple.serialize_element(child)?;
                    return tuple.end();
                }
            }
            _ => {}
        }

        self.context.serialize(serializer)
    }
}

impl Expression {
    pub fn simplify(&mut self) {
        match self {
            Expression::Seq(seq) if seq.len() == 1 => {
                *self = seq.pop().unwrap();
                self.simplify();
            }
            Expression::Seq(seq) => {
                seq.iter_mut().for_each(Expression::simplify);
            }
            Expression::Let { bindings, expr } => {
                expr.simplify();

                match (&bindings[..], &**expr) {
                    ([binding], Self::Get { name }) if &binding.name == name => {
                        *self = bindings.pop().unwrap().expr;
                        self.simplify();
                    }
                    _ => bindings.iter_mut().for_each(|binding| binding.expr.simplify()),
                }
            }
            Expression::Concat(vec) if vec.len() == 1 => {
                *self = vec.pop().unwrap();
                self.simplify();
            }
            Expression::Concat(vec) => {
                vec.iter_mut().for_each(Expression::simplify);
            }
            Expression::Sum(vec) if vec.len() == 1 => {
                *self = vec.pop().unwrap();
                self.simplify();
            }
            Expression::Sum(vec) => {
                vec.iter_mut().for_each(Expression::simplify);
            }
            Expression::Value(_) => {}
            Expression::Get { .. } => {}
            Expression::GetFirstNonEmpty { .. } => {}
            Expression::Query(_) => {}
            Expression::Execute(_) => {}
            Expression::Unique(expr) => {
                expr.simplify();
            }
            Expression::Required(expr) => {
                expr.simplify();
            }
            Expression::Join { parent, children, .. } => {
                parent.simplify();
                children.iter_mut().for_each(|child| child.child.simplify());
            }
            Expression::MapField { records, .. } => {
                records.simplify();
            }
            Expression::Transaction(expr) => {
                expr.simplify();
            }
            Expression::DataMap { expr, .. } => {
                expr.simplify();
            }
            Expression::RawNestedRead { .. } => {}
            Expression::Validate { expr, .. } => {
                expr.simplify();
            }
            Expression::If {
                value, then, r#else, ..
            } => {
                value.simplify();
                then.simplify();
                r#else.simplify();
            }
            Expression::Unit => {}
            Expression::Diff { from, to, .. } => {
                from.simplify();
                to.simplify();
            }
            Expression::InitializeRecord { expr, .. } => {
                expr.simplify();
            }
            Expression::MapRecord { expr, .. } => {
                expr.simplify();
            }
            Expression::Process { expr, .. } => {
                expr.simplify();
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum FieldInitializer {
    LastInsertId,
    Value(PrismaValue),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum FieldOperation {
    Set(PrismaValue),
    Add(PrismaValue),
    Subtract(PrismaValue),
    Multiply(PrismaValue),
    Divide(PrismaValue),
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

#[derive(Debug)]
pub struct Pagination {
    cursor: Option<HashMap<String, PrismaValue>>,
    take: Option<i64>,
    skip: Option<i64>,
}

impl Serialize for Pagination {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len =
            usize::from(self.cursor.is_some()) + usize::from(self.take.is_some()) + usize::from(self.skip.is_some());
        let mut map = serializer.serialize_map(Some(len))?;
        if let Some(cursor) = &self.cursor {
            map.serialize_entry("cursor", cursor)?;
        }
        if let Some(take) = self.take {
            map.serialize_entry("take", &take)?;
        }
        if let Some(skip) = self.skip {
            map.serialize_entry("skip", &skip)?;
        }
        map.end()
    }
}

#[bon]
impl Pagination {
    #[builder]
    pub fn new(cursor: Option<HashMap<String, PrismaValue>>, take: Option<i64>, skip: Option<i64>) -> Self {
        Self { cursor, take, skip }
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

#[derive(Debug, Default, Builder)]
pub struct InMemoryOps {
    pub(crate) pagination: Option<Pagination>,
    pub(crate) distinct: Option<Vec<String>>,
    #[builder(default)]
    pub(crate) reverse: bool,
    #[builder(default)]
    pub(crate) nested: BTreeMap<String, InMemoryOps>,
    pub(crate) linking_fields: Option<Vec<String>>,
}

impl Serialize for InMemoryOps {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = usize::from(self.pagination.is_some())
            + usize::from(self.distinct.is_some())
            + usize::from(self.reverse)
            + usize::from(!self.nested.is_empty())
            + usize::from(self.linking_fields.is_some());
        let mut map = serializer.serialize_map(Some(len))?;
        if let Some(pagination) = &self.pagination {
            map.serialize_entry("pagination", pagination)?;
        }
        if let Some(distinct) = &self.distinct {
            map.serialize_entry("distinct", distinct)?;
        }
        if self.reverse {
            map.serialize_entry("reverse", &self.reverse)?;
        }
        if !self.nested.is_empty() {
            map.serialize_entry("nested", &self.nested)?;
        }
        if let Some(linking_fields) = &self.linking_fields {
            map.serialize_entry("linkingFields", linking_fields)?;
        }
        map.end()
    }
}

impl InMemoryOps {
    pub fn is_empty(&self) -> bool {
        self.is_empty_toplevel() && self.nested.is_empty()
    }

    pub fn is_empty_toplevel(&self) -> bool {
        self.pagination.is_none() && self.distinct.is_none() && !self.reverse
    }

    pub fn into_expression(self, inner: Expression) -> Expression {
        if self.is_empty() {
            inner
        } else {
            Expression::Process {
                expr: inner.into(),
                operations: self,
            }
        }
    }
}

impl From<Pagination> for InMemoryOps {
    fn from(pagination: Pagination) -> Self {
        Self::builder().pagination(pagination).build()
    }
}

#[derive(Debug, Default, Serialize)]
pub struct EnumsMap(BTreeMap<String, BTreeMap<String, String>>);

impl EnumsMap {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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
            PrismaValueType::List(inner) => ExpressionType::List(Box::new(ExpressionType::from_value_type(*inner))),
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
            Expression::RawNestedRead { unique, .. } => {
                if *unique {
                    ExpressionType::Record
                } else {
                    ExpressionType::List(Box::new(ExpressionType::Record))
                }
            }
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
            Expression::InitializeRecord { .. } | Expression::MapRecord { .. } => ExpressionType::Record,
            Expression::Process { expr, .. } => expr.r#type(),
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
