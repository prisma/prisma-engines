use pretty::{
    termcolor::{Color, ColorSpec},
    DocAllocator, DocBuilder,
};
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
        let doc = self.to_doc(&arena);

        let mut buf = if color {
            pretty::termcolor::Buffer::ansi()
        } else {
            pretty::termcolor::Buffer::no_color()
        };

        doc.render_colored(width, &mut buf)?;
        Ok(String::from_utf8(buf.into_inner())?)
    }

    fn to_doc<'a, D>(&'a self, d: &'a D) -> DocBuilder<'a, D, ColorSpec>
    where
        D: DocAllocator<'a, ColorSpec>,
        D::Doc: Clone,
    {
        let color_kw = || ColorSpec::new().set_fg(Some(Color::Blue)).clone();
        let color_fn = || ColorSpec::new().set_underline(true).clone();
        let color_var = || ColorSpec::new().set_bold(true).clone();
        let color_lit = || ColorSpec::new().set_italic(true).set_fg(Some(Color::Green)).clone();

        let format_query = |tag: &'static str, db_query: &'a DbQuery| {
            d.text(tag)
                .annotate(color_kw())
                .append(d.softline())
                .append(
                    d.reflow(&db_query.query)
                        .align()
                        .enclose("«", "»")
                        .annotate(color_lit()),
                )
                .append(d.line())
                .append(d.text("with params").annotate(color_kw()))
                .append(d.space())
                .append(
                    d.intersperse(
                        db_query.params.iter().map(|param| match param {
                            PrismaValue::Placeholder { name, r#type } => d.text("var").annotate(color_kw()).append(
                                d.text(name)
                                    .annotate(color_var())
                                    .append(d.space())
                                    .append(d.text("as").annotate(color_kw()))
                                    .append(d.space())
                                    .append(match r#type {
                                        query_structure::PlaceholderType::Array(inner) => format!("{inner:?}[]"),
                                        _ => format!("{type:?}"),
                                    })
                                    .parens(),
                            ),
                            _ => d
                                .text("const")
                                .annotate(color_kw())
                                .append(d.text(format!("{param:?}")).annotate(color_lit()).parens()),
                        }),
                        d.text(",").append(d.softline()),
                    )
                    .align()
                    .brackets(),
                )
                .align()
        };

        let format_function = |name: &'static str, args: &'a [Expression]| {
            d.text(name).annotate(color_fn()).append(d.space()).append(
                d.intersperse(args.iter().map(|expr| expr.to_doc(d)), d.space())
                    .parens(),
            )
        };

        let format_unary_function = |name: &'static str, arg: &'a Expression| {
            d.text(name)
                .annotate(color_fn())
                .append(d.space())
                .append(arg.to_doc(d).parens())
        };

        match self {
            Expression::Seq(vec) => d.intersperse(vec.iter().map(|expr| expr.to_doc(d)), d.text(";").append(d.line())),

            Expression::Get { name } => d
                .text("get")
                .annotate(color_kw())
                .append(d.space())
                .append(d.text(name).annotate(color_var())),

            Expression::Let { bindings, expr } => d
                .text("let")
                .annotate(color_kw())
                .append(d.softline())
                .append(
                    d.intersperse(
                        bindings.iter().map(|binding| {
                            d.text(&binding.name)
                                .annotate(color_var())
                                .append(d.space())
                                .append("=")
                                .append(d.softline())
                                .append(binding.expr.to_doc(d))
                        }),
                        d.text(";").append(d.line()),
                    )
                    .align(),
                )
                .append(d.line())
                .append(d.text("in").annotate(color_kw()))
                .append(d.softline())
                .append(expr.to_doc(d).align()),

            Expression::GetFirstNonEmpty { names } => d
                .text("getFirstNonEmpty")
                .annotate(color_fn())
                .append(d.intersperse(names.iter().map(|name| d.text(name).annotate(color_var())), d.space())),

            Expression::Query(db_query) => format_query("query", db_query),

            Expression::Execute(db_query) => format_query("execute", db_query),

            Expression::Reverse(expression) => format_unary_function("reverse", expression),

            Expression::Sum(vec) => format_function("sum", vec),

            Expression::Concat(vec) => format_function("concat", vec),

            Expression::Unique(expression) => format_unary_function("unique", expression),

            Expression::Required(expression) => format_unary_function("required", expression),

            Expression::Join { parent, children } => d
                .text("join")
                .annotate(color_kw())
                .append(d.space())
                .append(parent.to_doc(d).parens())
                .append(d.line())
                .append(d.text("with").annotate(color_kw()))
                .append(d.space())
                .append(
                    d.intersperse(
                        children.iter().map(|join| {
                            join.child
                                .to_doc(d)
                                .parens()
                                .append(d.space())
                                .append(d.text("on").annotate(color_kw()))
                                .append(d.space())
                                .append(d.intersperse(
                                    join.on.iter().map(|(l, r)| {
                                        d.text("left")
                                            .annotate(color_kw())
                                            .append(".")
                                            .append(d.text(l).annotate(color_var()))
                                            .parens()
                                            .append(d.space())
                                            .append("=")
                                            .append(d.space())
                                            .append(
                                                d.text("right")
                                                    .annotate(color_kw())
                                                    .append(".")
                                                    .append(d.text(r).annotate(color_var()))
                                                    .parens(),
                                            )
                                    }),
                                    d.text(", "),
                                ))
                        }),
                        d.text(",").append(d.line()),
                    )
                    .align(),
                ),

            Expression::MapField { field, records } => d
                .text("mapField")
                .annotate(color_fn())
                .append(d.space())
                .append(d.text(field).double_quotes().annotate(color_lit()))
                .append(d.space())
                .append(records.to_doc(d).parens()),
        }
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pretty_print(false, 80).map_err(|_| std::fmt::Error)?.fmt(f)
    }
}
