use query_builder::DbQuery;
use serde::Serialize;

mod format;

#[derive(Debug, Serialize)]
pub struct Binding {
    pub name: String,
    pub expr: Expression,
}

impl Binding {
    pub fn new(name: String, expr: Expression) -> Self {
        Self { name, expr }
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
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "args", rename_all = "camelCase")]
pub enum Expression {
    /// Sequence of statements. The whole sequence evaluates to the result of the last expression.
    Seq(Vec<Expression>),

    /// Get binding value.
    Get { name: String },

    /// A lexical scope with let-bindings.
    Let {
        bindings: Vec<Binding>,
        expr: Box<Expression>,
    },

    /// Gets the first non-empty value from a list of bindings.
    GetFirstNonEmpty { names: Vec<String> },

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
}

#[derive(Debug, Clone)]
pub enum ExpressionType {
    Scalar,
    Record,
    List(Box<ExpressionType>),
    Dynamic,
}

impl ExpressionType {
    pub fn is_list(&self) -> bool {
        matches!(self, ExpressionType::List(_) | ExpressionType::Dynamic)
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
        }
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pretty_print(false, 80).map_err(|_| std::fmt::Error)?.fmt(f)
    }
}
