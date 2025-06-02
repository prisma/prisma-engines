use super::{Binding, DbQuery, Expression, JoinExpression, Pagination, RecordValue};
use crate::result_node::ResultNode;
use pretty::{
    DocAllocator, DocBuilder,
    termcolor::{Color, ColorSpec},
};
use query_core::DataRule;
use query_structure::{Placeholder, PrismaValue};
use std::{borrow::Cow, collections::BTreeMap};

fn color_kw() -> ColorSpec {
    ColorSpec::new().set_fg(Some(Color::Blue)).clone()
}

fn color_fn() -> ColorSpec {
    ColorSpec::new().set_underline(true).clone()
}

fn color_var() -> ColorSpec {
    ColorSpec::new().set_bold(true).clone()
}

fn color_lit() -> ColorSpec {
    ColorSpec::new().set_italic(true).set_fg(Some(Color::Green)).clone()
}

fn color_field() -> ColorSpec {
    ColorSpec::new().set_bold(true).set_fg(Some(Color::Yellow)).clone()
}

pub(super) struct PrettyPrinter<'a, D> {
    allocator: &'a D,
}

impl<'a, D> PrettyPrinter<'a, D>
where
    D: DocAllocator<'a, ColorSpec>,
    D::Doc: Clone,
{
    pub fn new(allocator: &'a D) -> Self {
        Self { allocator }
    }

    pub fn expression(&'a self, expression: &'a Expression) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        match expression {
            Expression::Seq(vec) => self.seq(vec),
            Expression::Get { name, .. } => self.get(name),
            Expression::Let { bindings, expr } => self.r#let(bindings, expr),
            Expression::GetFirstNonEmpty { names } => self.get_first_non_empty(names),
            Expression::Query(db_query) => self.query("query", db_query),
            Expression::Execute(db_query) => self.query("execute", db_query),
            Expression::Reverse(expression) => self.unary_function("reverse", expression),
            Expression::Sum(vec) => self.function("sum", vec),
            Expression::Concat(vec) => self.function("concat", vec),
            Expression::Unique(expression) => self.unary_function("unique", expression),
            Expression::Required(expression) => self.unary_function("required", expression),
            Expression::Join { parent, children } => self.join(parent, children),
            Expression::MapField { field, records } => self.map_field(field, records),
            Expression::Transaction(expression) => self.transaction(expression),
            Expression::DataMap { expr, structure } => self.data_map(expr, structure),
            Expression::Validate {
                expr,
                rules,
                error_identifier,
                ..
            } => self.validate(expr, rules, error_identifier),
            Expression::If {
                value,
                rule,
                then,
                r#else,
            } => self.r#if(value, rule, then, r#else),
            Expression::Unit => self.keyword("()"),
            Expression::Diff { from, to } => self.diff(from, to),
            Expression::DistinctBy { expr, fields } => self.distinct_by(expr, fields),
            Expression::Paginate { expr, pagination } => self.paginate(expr, pagination),
            Expression::ExtendRecord { expr, values } => self.extend_record(expr, values),
        }
    }

    fn keyword(&'a self, keyword: &'static str) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.text(keyword).annotate(color_kw())
    }

    fn var_name(&'a self, name: &'a str) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.text(name).annotate(color_var())
    }

    fn field_name(&'a self, name: &'a str) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.text(name).annotate(color_field())
    }

    fn tuple(
        &'a self,
        subtrees: impl IntoIterator<Item = DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec>>,
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.intersperse(subtrees, self.text(",").append(self.softline()))
            .align()
            .parens()
    }

    fn query(&'a self, tag: &'static str, db_query: &'a DbQuery) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        let sql = db_query.to_string();

        // Copied the implementation from reflow, because DocBuilder does not provide the API to avoid issues with lifetimes here
        let fragments = sql.split_whitespace().map(|word| Cow::<'a, str>::from(word.to_owned()));

        let doc_builder = self
            .intersperse(fragments, self.softline()) // Replacement for: .reflow(&sql)
            .align()
            .enclose("«", "»")
            .annotate(color_lit());

        self.keyword(tag)
            .append(self.softline())
            .append(doc_builder)
            .append(self.line())
            .append(self.keyword("params"))
            .append(self.space())
            .append(self.list(db_query.params()))
            .align()
    }

    fn list(&'a self, values: &'a [PrismaValue]) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.intersperse(
            values.iter().map(|value| self.value(value)),
            self.text(",").append(self.softline()),
        )
        .align()
        .brackets()
    }

    fn value(&'a self, value: &'a PrismaValue) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        match value {
            PrismaValue::Placeholder(Placeholder { name, r#type }) => self.keyword("var").append(
                self.var_name(name)
                    .append(self.space())
                    .append(self.keyword("as"))
                    .append(self.space())
                    .append(match r#type {
                        query_structure::PrismaValueType::Array(inner) => format!("{inner:?}[]"),
                        _ => format!("{type:?}"),
                    })
                    .parens(),
            ),
            PrismaValue::GeneratorCall { name, args, .. } => self
                .var_name(name)
                .append(self.tuple(args.iter().map(|arg| self.value(arg)))),
            PrismaValue::List(values) => self.list(values),
            _ => self
                .keyword("const")
                .append(self.text(format!("{value:?}")).annotate(color_lit()).parens()),
        }
    }

    fn function(
        &'a self,
        name: &'static str,
        args: impl IntoIterator<Item = &'a Expression>,
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.text(name)
            .annotate(color_fn())
            .append(self.softline())
            .append(self.intersperse(
                args.into_iter().map(|expr| self.expression(expr).parens().align()),
                self.softline(),
            ))
    }

    fn unary_function(
        &'a self,
        name: &'static str,
        arg: &'a Expression,
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.text(name)
            .annotate(color_fn())
            .append(self.space())
            .append(self.expression(arg).parens())
    }

    fn seq(&'a self, vec: &'a [Expression]) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.intersperse(
            vec.iter().map(|expr| self.expression(expr)),
            self.text(";").append(self.line()),
        )
    }

    fn get(&'a self, name: &'a str) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.keyword("get").append(self.space()).append(self.var_name(name))
    }

    fn r#let(
        &'a self,
        bindings: &'a [Binding],
        expr: &'a Expression,
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.keyword("let")
            .append(self.softline())
            .append(
                self.intersperse(
                    bindings.iter().map(|binding| {
                        self.var_name(&binding.name)
                            .append(self.space())
                            .append("=")
                            .append(self.softline())
                            .append(self.expression(&binding.expr))
                    }),
                    self.text(";").append(self.line()),
                )
                .align(),
            )
            .append(self.line())
            .append(self.keyword("in"))
            .append(self.softline())
            .append(self.expression(expr).align())
    }

    fn get_first_non_empty(
        &'a self,
        names: &'a [Cow<'static, str>],
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.text("getFirstNonEmpty")
            .annotate(color_fn())
            .append(self.intersperse(names.iter().map(|name| self.var_name(name)), self.space()))
    }

    fn join(
        &'a self,
        parent: &'a Expression,
        children: &'a [JoinExpression],
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.keyword("join")
            .append(self.space())
            .append(self.expression(parent).parens())
            .append(self.line())
            .append(self.keyword("with"))
            .append(self.space())
            .append(
                self.intersperse(
                    children.iter().map(|join| {
                        let (left_fields, right_fields): (Vec<_>, Vec<_>) = join
                            .on
                            .iter()
                            .map(|(l, r)| (self.field_name(l), self.field_name(r)))
                            .unzip();
                        let mut builder = self
                            .expression(&join.child)
                            .parens()
                            .append(self.space())
                            .append(self.keyword("on"));

                        if join.is_relation_unique {
                            builder = builder.append(self.space()).append(self.keyword("unique"));
                        }

                        builder
                            .append(self.space())
                            .append(
                                self.keyword("left")
                                    .append(".")
                                    .append(self.tuple(left_fields))
                                    .append(self.space())
                                    .append("=")
                                    .append(self.space())
                                    .append(self.keyword("right"))
                                    .append(".")
                                    .append(self.tuple(right_fields)),
                            )
                            .append(self.space())
                            .append(self.keyword("as"))
                            .append(self.space())
                            .append(self.field_name(&join.parent_field))
                    }),
                    self.text(",").append(self.line()),
                )
                .align(),
            )
    }

    fn map_field(&'a self, field: &'a str, records: &'a Expression) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.text("mapField")
            .annotate(color_fn())
            .append(self.space())
            .append(self.field_name(field))
            .append(self.space())
            .append(self.expression(records).parens())
    }

    fn transaction(&'a self, expr: &'a Expression) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.keyword("transaction")
            .append(self.line())
            .append(self.softline())
            .append(self.softline())
            .append(self.softline())
            .append(self.expression(expr).align())
    }

    fn data_map(
        &'a self,
        expr: &'a Expression,
        structure: &'a ResultNode,
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.keyword("dataMap")
            .append(self.space())
            .append(self.data_map_node(structure))
            .append(self.line())
            .append(self.expression(expr))
    }

    fn data_map_node(&'a self, node: &'a ResultNode) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        match node {
            ResultNode::AffectedRows => self.keyword("affectedRows"),
            ResultNode::Object { fields, flattened } => self
                .line()
                .append(
                    self.intersperse(
                        fields.iter().map(|(name, field)| {
                            let doc = self.field_name(name);
                            let doc = if *flattened {
                                doc.append(self.space().append(self.keyword("(flattened)")))
                            } else {
                                doc
                            };
                            doc.append(self.text(":"))
                                .append(self.space())
                                .append(self.data_map_node(field))
                        }),
                        self.line(),
                    )
                    .append(self.line())
                    .indent(4),
                )
                .braces(),
            ResultNode::Value { db_name, result_type } => self
                .text(result_type.to_string())
                .append(self.space())
                .append(self.field_name(db_name).parens()),
        }
    }

    fn validate(
        &'a self,
        expr: &'a Expression,
        rules: &'a [DataRule],
        id: &'a str,
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.keyword("validate")
            .append(self.softline())
            .append(self.expression(expr).align().parens())
            .append(self.line())
            .append(
                self.intersperse(
                    rules.iter().map(|rule| {
                        let rendered_rule = match rule {
                            DataRule::RowCountEq(count) => self
                                .text("rowCountEq")
                                .append(self.softline())
                                .append(self.text(count.to_string())),
                            DataRule::RowCountNeq(count) => self
                                .text("rowCountNeq")
                                .append(self.softline())
                                .append(self.text(count.to_string())),
                            DataRule::AffectedRowCountEq(count) => self
                                .text("affectedRowCountEq")
                                .append(self.softline())
                                .append(self.text(count.to_string())),
                            DataRule::Never => self.text("never"),
                        };
                        self.softline().append(rendered_rule).append(self.line())
                    }),
                    self.text(",").append(self.line()),
                )
                .brackets(),
            )
            .append(self.softline())
            .append(self.keyword("orRaise"))
            .append(self.softline())
            .append(self.text(format!("{id:?}")))
    }

    fn r#if(
        &'a self,
        value: &'a Expression,
        rule: &'a DataRule,
        then: &'a Expression,
        r#else: &'a Expression,
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.keyword("if")
            .append(self.softline())
            .append(
                self.text(rule.to_string())
                    .append(self.softline())
                    .append(self.expression(value).parens().align())
                    .parens(),
            )
            .append(self.line())
            .append(self.keyword("then"))
            .append(self.softline())
            .append(self.expression(then).align())
            .append(self.line())
            .append(self.keyword("else"))
            .append(self.softline())
            .append(self.expression(r#else).align())
    }

    fn diff(&'a self, from: &'a Expression, to: &'a Expression) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.function("diff", [from, to])
    }

    fn distinct_by(
        &'a self,
        expr: &'a Expression,
        fields: &'a [String],
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.keyword("distinct")
            .append(self.softline())
            .append(self.keyword("by"))
            .append(self.softline())
            .append(self.tuple(fields.iter().map(|name| self.var_name(name))))
            .append(self.line())
            .append(self.expression(expr).parens())
    }

    fn paginate(
        &'a self,
        expr: &'a Expression,
        pagination: &'a Pagination,
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        let mut builder = self.nil();

        if let Some(fields) = &pagination.cursor {
            builder = builder.append(
                self.keyword("cursor").append(self.softline()).append(
                    self.intersperse(
                        fields
                            .iter()
                            .map(|(name, val)| self.tuple([self.text(format!("{name:?}")), self.value(val)])),
                        self.text(",").append(self.softline()),
                    )
                    .align()
                    .brackets()
                    .append(self.line()),
                ),
            );
        }

        if let Some(skip) = &pagination.skip() {
            builder = builder.append(
                self.keyword("skip")
                    .append(self.space())
                    .append(self.text(skip.to_string()))
                    .append(self.line()),
            );
        }

        if let Some(take) = &pagination.take() {
            builder = builder.append(
                self.keyword("take")
                    .append(self.space())
                    .append(self.text(take.to_string()))
                    .append(self.line()),
            );
        }

        builder.append(self.expression(expr))
    }

    fn extend_record(
        &'a self,
        expr: &'a Expression,
        fields: &'a BTreeMap<String, RecordValue>,
    ) -> DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec> {
        self.keyword("extend")
            .append(self.space())
            .append(
                self.intersperse(
                    fields.iter().map(|(name, value)| {
                        let value = match value {
                            RecordValue::LastInsertId => self.keyword("lastInsertId"),
                            RecordValue::Value(value) => self.value(value),
                        };
                        self.field_name(name)
                            .append(self.text(":"))
                            .append(self.space())
                            .append(value)
                    }),
                    self.line(),
                )
                .append(self.line())
                .indent(4)
                .braces(),
            )
            .append(self.space())
            .append(self.expression(expr))
    }
}

impl<'a, D, A> DocAllocator<'a, A> for PrettyPrinter<'a, D>
where
    D: DocAllocator<'a, A>,
    A: 'a,
{
    type Doc = D::Doc;

    fn alloc(&'a self, doc: pretty::Doc<'a, Self::Doc, A>) -> Self::Doc {
        self.allocator.alloc(doc)
    }

    fn alloc_column_fn(
        &'a self,
        f: impl Fn(usize) -> Self::Doc + 'a,
    ) -> <Self::Doc as pretty::DocPtr<'a, A>>::ColumnFn {
        self.allocator.alloc_column_fn(f)
    }

    fn alloc_width_fn(&'a self, f: impl Fn(isize) -> Self::Doc + 'a) -> <Self::Doc as pretty::DocPtr<'a, A>>::WidthFn {
        self.allocator.alloc_width_fn(f)
    }
}
