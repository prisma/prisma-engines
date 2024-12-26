use itertools::Itertools;
use query_structure::PrismaValue;
use serde::Serialize;

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
pub struct DbQuery {
    pub query: String,
    pub params: Vec<PrismaValue>,
}

impl DbQuery {
    pub fn new(query: String, params: Vec<PrismaValue>) -> Self {
        Self { query, params }
    }
}

#[derive(Debug, Serialize)]
pub struct JoinExpression {
    pub child: Expression,
    pub on: Vec<(String, String)>,
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

impl Expression {
    fn display(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        let indent = "  ".repeat(level);

        match self {
            Self::Seq(exprs) => {
                writeln!(f, "{indent}{{")?;
                for expr in exprs {
                    expr.display(f, level + 1)?;
                    writeln!(f, ";")?;
                }
                write!(f, "{indent}}}")?;
            }

            Self::Get { name } => {
                write!(f, "{indent}get {name}")?;
            }

            Self::Let { bindings, expr } => {
                writeln!(f, "{indent}let")?;
                for Binding { name, expr } in bindings {
                    writeln!(f, "{indent}  {name} =")?;
                    expr.display(f, level + 2)?;
                    writeln!(f, ";")?;
                }
                writeln!(f, "{indent}in")?;
                expr.display(f, level + 1)?;
            }

            Self::GetFirstNonEmpty { names } => {
                write!(f, "{indent}getFirstNonEmpty")?;
                for name in names {
                    write!(f, " {}", name)?;
                }
            }

            Self::Query(query) => self.display_query("query", query, f, level)?,

            Self::Execute(query) => self.display_query("execute", query, f, level)?,

            Self::Reverse(expr) => {
                writeln!(f, "{indent}reverse (")?;
                expr.display(f, level + 1)?;
                write!(f, "{indent})")?;
            }

            Self::Sum(exprs) => self.display_function("sum", exprs, f, level)?,

            Self::Concat(exprs) => self.display_function("concat", exprs, f, level)?,

            Self::Unique(expr) => {
                writeln!(f, "{indent}unique (")?;
                expr.display(f, level + 1)?;
                write!(f, "{indent})")?;
            }

            Self::Required(expr) => {
                writeln!(f, "{indent}required (")?;
                expr.display(f, level + 1)?;
                write!(f, "{indent})")?;
            }

            Self::Join { parent, children } => {
                writeln!(f, "{indent}join (")?;
                parent.display(f, level + 1)?;
                for nested in children {
                    let left = nested.on.iter().map(|(l, _)| l).cloned().join(", ");
                    let right = nested.on.iter().map(|(_, r)| r).cloned().join(", ");
                    writeln!(f, "\n{indent}  with (")?;
                    nested.child.display(f, level + 2)?;
                    writeln!(f, "\n{indent}  ) on left.{left} = right.{right},")?;
                }
                write!(f, "{indent})")?;
            }

            Self::MapField { field, records } => {
                writeln!(f, "{indent}mapField {field} (")?;
                records.display(f, level + 1)?;
                write!(f, "\n{indent})")?;
            }
        }

        Ok(())
    }

    fn display_query(
        &self,
        op: &str,
        db_query: &DbQuery,
        f: &mut std::fmt::Formatter<'_>,
        level: usize,
    ) -> std::fmt::Result {
        let indent = "  ".repeat(level);
        let DbQuery { query, params } = db_query;
        write!(f, "{indent}{op} (\n{indent}  {query}\n{indent}) with {params:?}")
    }

    fn display_function(
        &self,
        name: &str,
        args: &[Expression],
        f: &mut std::fmt::Formatter<'_>,
        level: usize,
    ) -> std::fmt::Result {
        let indent = "  ".repeat(level);
        write!(f, "{indent}{name} (")?;
        for arg in args {
            arg.display(f, level + 1)?;
            writeln!(f, ",")?;
        }
        write!(f, ")")
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display(f, 0)
    }
}
