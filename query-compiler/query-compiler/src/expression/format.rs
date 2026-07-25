use super::{
    Binding, DbQuery, EnumsMap, Expression, FieldOperation, JoinExpression, Pagination, RawNestedFinalOwnerSchedule,
    RawNestedReadQuery, RawNestedReadRelation, RawResultColumnRef, RawResultFieldName,
};
use crate::{
    expression::{FieldInitializer, InMemoryOps},
    result_node::ResultNode,
};
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

type PrettyDoc<'a, D> = DocBuilder<'a, PrettyPrinter<'a, D>, ColorSpec>;

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

    pub fn expression(&'a self, expression: &'a Expression) -> PrettyDoc<'a, D> {
        match expression {
            Expression::Value(value) => self.value(value),
            Expression::Seq(vec) => self.seq(vec),
            Expression::Get { name, .. } => self.get(name),
            Expression::Let { bindings, expr } => self.r#let(bindings, expr),
            Expression::GetFirstNonEmpty { names } => self.get_first_non_empty(names),
            Expression::Query(db_query) => self.query("query", db_query),
            Expression::Execute(db_query) => self.query("execute", db_query),
            Expression::Sum(vec) => self.function("sum", vec),
            Expression::Concat(vec) => self.function("concat", vec),
            Expression::Unique(expression) => self.unary_function("unique", expression),
            Expression::Required(expression) => self.unary_function("required", expression),
            Expression::Join { parent, children, .. } => self.join(parent, children),
            Expression::MapField { field, records } => self.map_field(field, records),
            Expression::Transaction(expression) => self.transaction(expression),
            Expression::DataMap { expr, structure, enums } => self.data_map(expr, structure, enums),
            Expression::RawNestedRead { query, unique, enums } => self.raw_nested_read(query, *unique, enums),
            Expression::Validate { expr, rules, error, .. } => self.validate(expr, rules, error.id()),
            Expression::If {
                value,
                rule,
                then,
                r#else,
            } => self.r#if(value, rule, then, r#else),
            Expression::Unit => self.keyword("()"),
            Expression::Diff { from, to, .. } => self.diff(from, to),
            Expression::InitializeRecord { expr, fields } => self.initialize_record(expr, fields),
            Expression::MapRecord { expr, fields } => self.map_record(expr, fields),
            Expression::Process { expr, operations } => self.process(expr, operations),
        }
    }

    fn keyword(&'a self, keyword: &'static str) -> PrettyDoc<'a, D> {
        self.text(keyword).annotate(color_kw())
    }

    fn var_name(&'a self, name: &'a str) -> PrettyDoc<'a, D> {
        self.text(name).annotate(color_var())
    }

    fn field_name(&'a self, name: &'a str) -> PrettyDoc<'a, D> {
        self.text(name).annotate(color_field())
    }

    fn tuple(&'a self, subtrees: impl IntoIterator<Item = PrettyDoc<'a, D>>) -> PrettyDoc<'a, D> {
        self.intersperse(subtrees, self.text(",").append(self.softline()))
            .align()
            .parens()
    }

    fn fields<F>(&'a self, fields: impl IntoIterator<Item = &'a F>) -> PrettyDoc<'a, D>
    where
        F: AsRef<str> + 'a,
    {
        self.tuple(fields.into_iter().map(|f| self.field_name(f.as_ref())))
    }

    fn query(&'a self, tag: &'static str, db_query: &'a DbQuery) -> PrettyDoc<'a, D> {
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

    fn list(&'a self, values: impl IntoIterator<Item = &'a PrismaValue>) -> PrettyDoc<'a, D> {
        self.intersperse(
            values.into_iter().map(|value| self.value(value)),
            self.text(",").append(self.softline()),
        )
        .align()
        .brackets()
    }

    fn value(&'a self, value: &'a PrismaValue) -> PrettyDoc<'a, D> {
        match value {
            PrismaValue::Placeholder(Placeholder { name, r#type }) => self.keyword("var").append(
                self.var_name(name)
                    .append(self.space())
                    .append(self.keyword("as"))
                    .append(self.space())
                    .append(match r#type {
                        query_structure::PrismaValueType::List(inner) => format!("{inner:?}[]"),
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

    fn function(&'a self, name: &'static str, args: impl IntoIterator<Item = &'a Expression>) -> PrettyDoc<'a, D> {
        self.text(name)
            .annotate(color_fn())
            .append(self.softline())
            .append(self.intersperse(
                args.into_iter().map(|expr| self.expression(expr).parens().align()),
                self.softline(),
            ))
    }

    fn unary_function(&'a self, name: &'static str, arg: &'a Expression) -> PrettyDoc<'a, D> {
        self.text(name)
            .annotate(color_fn())
            .append(self.space())
            .append(self.expression(arg).parens())
    }

    fn seq(&'a self, vec: &'a [Expression]) -> PrettyDoc<'a, D> {
        self.intersperse(
            vec.iter().map(|expr| self.expression(expr)),
            self.text(";").append(self.line()),
        )
    }

    fn get(&'a self, name: &'a str) -> PrettyDoc<'a, D> {
        self.keyword("get").append(self.space()).append(self.var_name(name))
    }

    fn r#let(&'a self, bindings: &'a [Binding], expr: &'a Expression) -> PrettyDoc<'a, D> {
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

    fn get_first_non_empty(&'a self, names: &'a [Cow<'static, str>]) -> PrettyDoc<'a, D> {
        self.text("getFirstNonEmpty")
            .annotate(color_fn())
            .append(self.intersperse(names.iter().map(|name| self.var_name(name)), self.space()))
    }

    fn join(&'a self, parent: &'a Expression, children: &'a [JoinExpression]) -> PrettyDoc<'a, D> {
        self.keyword("join")
            .append(self.space())
            .append(self.expression(parent).parens())
            .append(self.line())
            .append(self.keyword("with"))
            .append(self.space())
            .append(
                self.intersperse(
                    children.iter().map(|join| {
                        let (left_fields, right_fields): (Vec<_>, Vec<_>) = join.on.iter().map(|(l, r)| (l, r)).unzip();
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
                                    .append(self.fields(left_fields))
                                    .append(self.space())
                                    .append("=")
                                    .append(self.space())
                                    .append(self.keyword("right"))
                                    .append(".")
                                    .append(self.fields(right_fields)),
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

    fn map_field(&'a self, field: &'a str, records: &'a Expression) -> PrettyDoc<'a, D> {
        self.text("mapField")
            .annotate(color_fn())
            .append(self.space())
            .append(self.field_name(field))
            .append(self.space())
            .append(self.expression(records).parens())
    }

    fn transaction(&'a self, expr: &'a Expression) -> PrettyDoc<'a, D> {
        self.keyword("transaction")
            .append(self.line())
            .append(self.softline())
            .append(self.softline())
            .append(self.softline())
            .append(self.expression(expr).align())
    }

    fn data_map(&'a self, expr: &'a Expression, structure: &'a ResultNode, enums: &'a EnumsMap) -> PrettyDoc<'a, D> {
        let mut doc = self
            .keyword("dataMap")
            .append(self.space())
            .append(self.data_map_node(structure))
            .append(self.line());

        if !enums.0.is_empty() {
            doc = doc
                .append(self.keyword("enums"))
                .append(self.space())
                .append(self.enum_map(enums))
                .append(self.line());
        }

        doc.append(self.expression(expr))
    }

    fn raw_nested_read(&'a self, query: &'a RawNestedReadQuery, unique: bool, enums: &'a EnumsMap) -> PrettyDoc<'a, D> {
        let mut doc = self
            .keyword(if unique { "rawNestedReadUnique" } else { "rawNestedRead" })
            .append(self.space())
            .append(self.raw_nested_read_query(query));

        if !enums.0.is_empty() {
            doc = doc
                .append(self.line())
                .append(self.keyword("enums"))
                .append(self.space())
                .append(self.enum_map(enums));
        }

        doc
    }

    fn raw_nested_read_query(&'a self, query: &'a RawNestedReadQuery) -> PrettyDoc<'a, D> {
        let fields = self.object(query.fields.iter().map(|field| {
            let name = match &field.field_name {
                RawResultFieldName::Field(name) => self.field_name(name),
                RawResultFieldName::Path(path) => self
                    .intersperse(path.iter().map(|segment| self.field_name(segment)), self.text("."))
                    .align(),
            };
            (name, self.raw_result_column_ref(&field.column))
        }));

        let mut doc = self
            .query("query", &query.query)
            .append(self.line())
            .append(self.keyword("fields"))
            .append(self.space())
            .append(fields);

        if !query.relations.is_empty() {
            doc = doc
                .append(self.line())
                .append(self.keyword("relations"))
                .append(self.space())
                .append(self.raw_nested_relations(&query.relations));
        }

        if let Some(schedule) = &query.schedule {
            doc = doc
                .append(self.line())
                .append(self.keyword("schedule"))
                .append(self.space())
                .append(self.raw_nested_final_owner_schedule(schedule));
        }

        doc.align().parens()
    }

    fn raw_nested_final_owner_schedule(&'a self, schedule: &'a RawNestedFinalOwnerSchedule) -> PrettyDoc<'a, D> {
        self.keyword("finalOwner").append(self.space()).append(self.object([
            (
                self.field_name("rootKey"),
                self.raw_result_column_ref(&schedule.root_key_column),
            ),
            (
                self.field_name("unique"),
                self.text(format!(
                    "[{}, {}]",
                    schedule.unique_relations[0], schedule.unique_relations[1]
                )),
            ),
            (
                self.field_name("wrapperList"),
                self.text(format!(
                    "[{}, {}]",
                    schedule.wrapper_list.relation_index, schedule.wrapper_list.child_relation_index
                )),
            ),
            (
                self.field_name("childList"),
                self.text(format!(
                    "[{}, {}]",
                    schedule.child_list.relation_index, schedule.child_list.child_relation_index
                )),
            ),
        ]))
    }

    fn raw_nested_relations(&'a self, relations: &'a [RawNestedReadRelation]) -> PrettyDoc<'a, D> {
        self.intersperse(
            relations.iter().map(|relation| match relation {
                RawNestedReadRelation::Direct(relation) => self
                    .field_name(&relation.field_name)
                    .append(self.space())
                    .append(self.keyword("on"))
                    .append(self.space())
                    .append(self.keyword("left"))
                    .append(".")
                    .append(self.raw_result_column_ref(&relation.parent_column))
                    .append(self.space())
                    .append("=")
                    .append(self.space())
                    .append(self.keyword("right"))
                    .append(".")
                    .append(self.raw_result_column_ref(&relation.child_column))
                    .append(self.space())
                    .append(if relation.is_relation_unique {
                        self.keyword("unique")
                    } else {
                        self.keyword("many")
                    })
                    .append(self.space())
                    .append(self.raw_nested_read_query(&relation.child)),
            }),
            self.text(",").append(self.line()),
        )
        .align()
        .brackets()
    }

    fn raw_result_column_ref(&'a self, column: &'a RawResultColumnRef) -> PrettyDoc<'a, D> {
        match column {
            RawResultColumnRef::Index(index) => self.text(index.to_string()),
            RawResultColumnRef::Name(name) => self.field_name(name),
        }
    }

    fn data_map_node(&'a self, node: &'a ResultNode) -> PrettyDoc<'a, D> {
        match node {
            ResultNode::AffectedRows => self.keyword("affectedRows"),
            ResultNode::Object(object) => self.object(object.fields().map(|(name, field)| {
                let mut key = self.field_name(name);
                if let ResultNode::Object(nested_object) = field {
                    let source = match nested_object.serialized_name() {
                        Some(original_key) => self
                            .keyword("from")
                            .append(self.space())
                            .append(self.field_name(original_key)),
                        None => self.keyword("inlined"),
                    };
                    key = key.append(self.space().append(source.parens()))
                }
                (key, self.data_map_node(field))
            })),
            ResultNode::Field { db_name, field_type } => self
                .text(field_type.to_string())
                .append(self.space())
                .append(self.field_name(db_name).parens()),
        }
    }

    fn enum_map(&'a self, enums: &'a EnumsMap) -> PrettyDoc<'a, D> {
        self.object(enums.0.iter().map(|(enum_name, value_mapping)| {
            (
                self.text(enum_name),
                self.object(
                    value_mapping
                        .iter()
                        .map(|(db_name, prisma_name)| (self.text(db_name), self.text(prisma_name))),
                ),
            )
        }))
    }

    fn object(&'a self, pairs: impl IntoIterator<Item = (PrettyDoc<'a, D>, PrettyDoc<'a, D>)>) -> PrettyDoc<'a, D> {
        let pairs = pairs.into_iter().collect::<Vec<_>>();
        if pairs.is_empty() {
            return self.text("{}");
        }

        self.indented_braces(
            self.intersperse(
                pairs
                    .into_iter()
                    .map(|(key, value)| key.append(self.text(":").append(self.space()).append(value))),
                self.line(),
            ),
        )
    }

    fn indented_braces(&'a self, content: PrettyDoc<'a, D>) -> PrettyDoc<'a, D> {
        self.line().append(content.append(self.line()).indent(4)).braces()
    }

    fn validate(&'a self, expr: &'a Expression, rules: &'a [DataRule], id: &'a str) -> PrettyDoc<'a, D> {
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
    ) -> PrettyDoc<'a, D> {
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

    fn diff(&'a self, from: &'a Expression, to: &'a Expression) -> PrettyDoc<'a, D> {
        self.function("diff", [from, to])
    }

    fn initialize_record(
        &'a self,
        expr: &'a Expression,
        fields: &'a BTreeMap<String, FieldInitializer>,
    ) -> PrettyDoc<'a, D> {
        self.keyword("initRecord")
            .append(self.space())
            .append(self.object(fields.iter().map(|(name, value)| {
                (
                    self.field_name(name),
                    match value {
                        FieldInitializer::LastInsertId => self.keyword("lastInsertId"),
                        FieldInitializer::Value(prisma_value) => self.value(prisma_value),
                    },
                )
            })))
            .append(self.space())
            .append(self.expression(expr))
    }

    fn map_record(&'a self, expr: &'a Expression, fields: &'a BTreeMap<String, FieldOperation>) -> PrettyDoc<'a, D> {
        self.keyword("mapRecord")
            .append(self.space())
            .append(self.object(fields.iter().map(|(name, value)| {
                (
                    self.field_name(name),
                    match value {
                        FieldOperation::Set(value) => {
                            self.keyword("set").append(self.space()).append(self.value(value))
                        }
                        FieldOperation::Add(val) => self.keyword("add").append(self.space()).append(self.value(val)),
                        FieldOperation::Subtract(val) => {
                            self.keyword("sub").append(self.space()).append(self.value(val))
                        }
                        FieldOperation::Multiply(val) => {
                            self.keyword("mul").append(self.space()).append(self.value(val))
                        }
                        FieldOperation::Divide(val) => self.keyword("div").append(self.space()).append(self.value(val)),
                    },
                )
            })))
            .append(self.space())
            .append(self.expression(expr))
    }

    fn process(&'a self, expr: &'a Expression, ops: &'a InMemoryOps) -> PrettyDoc<'a, D> {
        self.keyword("process")
            .append(self.space())
            .append(self.in_memory_ops_recursive(ops))
            .append(self.space())
            .append(self.expression(expr).parens())
    }

    fn in_memory_ops_recursive(&'a self, ops: &'a InMemoryOps) -> PrettyDoc<'a, D> {
        self.object(
            (!ops.is_empty_toplevel())
                .then(|| (self.text("."), self.in_memory_ops_single(ops)))
                .into_iter()
                .chain(
                    ops.nested
                        .iter()
                        .map(|(name, ops)| (self.text(name), self.in_memory_ops_recursive(ops))),
                ),
        )
    }

    fn in_memory_ops_single(&'a self, ops: &'a InMemoryOps) -> PrettyDoc<'a, D> {
        self.object(
            ops.linking_fields
                .as_ref()
                .map(|fields| (self.text("linkingFields"), self.fields(fields)))
                .into_iter()
                .chain(
                    ops.distinct
                        .as_ref()
                        .map(|fields| (self.text("distinct"), self.fields(fields))),
                )
                .chain(ops.reverse.then_some((self.text("reverse"), self.keyword("true"))))
                .chain(
                    ops.pagination
                        .as_ref()
                        .map(|pagination| (self.text("pagination"), self.pagination(pagination))),
                ),
        )
    }

    fn pagination(&'a self, pagination: &'a Pagination) -> PrettyDoc<'a, D> {
        self.object(
            pagination
                .skip()
                .map(|skip| (self.text("skip"), self.text(skip.to_string())))
                .into_iter()
                .chain(
                    pagination
                        .take()
                        .map(|take| (self.text("take"), self.text(take.to_string()))),
                )
                .chain(pagination.cursor().map(|cursor| {
                    (
                        self.text("cursor"),
                        self.object(
                            cursor
                                .iter()
                                .map(|(field, val)| (self.field_name(field), self.value(val))),
                        ),
                    )
                })),
        )
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
