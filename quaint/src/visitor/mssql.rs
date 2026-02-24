use super::{NativeColumnType, Visitor};
use crate::ast::Update;
use crate::prelude::{JsonArrayAgg, JsonBuildObject, JsonExtract, JsonType, JsonUnquote};
use crate::visitor::query_writer::QueryWriter;
use crate::{
    Value, ValueType,
    ast::{
        Column, Comparable, Expression, ExpressionKind, Insert, IntoRaw, Join, JoinData, Joinable, Merge, OnConflict,
        Order, Ordering, Row, Table, TypeDataLength, TypeFamily, Values,
    },
    error::{Error, ErrorKind},
    prelude::{Aliasable, Average, Query},
    visitor,
};
use either::Either;
use itertools::Itertools;
use query_template::{PlaceholderFormat, QueryTemplate};
use std::{borrow::Cow, convert::TryFrom, iter};

static GENERATED_KEYS: &str = "@generated_keys";

/// A visitor to generate queries for the SQL Server database.
///
/// The returned parameter values can be used directly with the tiberius crate.
pub struct Mssql<'a> {
    query_template: QueryTemplate<Value<'a>>,
    order_by_set: bool,
}

impl<'a> Mssql<'a> {
    /// Expression that evaluates to the current MSSQL server version.
    pub const fn version_expr() -> &'static str {
        "@@VERSION"
    }

    // TODO: figure out that merge shit
    fn visit_returning(&mut self, columns: Vec<Column<'a>>) -> visitor::Result {
        let cols: Vec<_> = columns.into_iter().map(|c| c.table("Inserted")).collect();

        self.write(" OUTPUT ")?;

        let len = cols.len();
        for (i, value) in cols.into_iter().enumerate() {
            self.visit_column(value)?;

            if i < (len - 1) {
                self.write(",")?;
            }
        }

        self.write(" INTO ")?;
        self.write(GENERATED_KEYS)?;

        Ok(())
    }

    fn visit_type_family(&mut self, type_family: TypeFamily) -> visitor::Result {
        match type_family {
            TypeFamily::Text(len) => {
                self.write("NVARCHAR(")?;
                match len {
                    Some(TypeDataLength::Constant(len)) => self.write(len)?,
                    Some(TypeDataLength::Maximum) => self.write("MAX")?,
                    None => self.write(4000)?,
                }
                self.write(")")
            }
            TypeFamily::Int => self.write("BIGINT"),
            TypeFamily::Float => self.write("FLOAT(24)"),
            TypeFamily::Double => self.write("FLOAT(53)"),
            TypeFamily::Decimal(size) => {
                self.write("DECIMAL(")?;
                match size {
                    Some((p, s)) => {
                        self.write(p)?;
                        self.write(",")?;
                        self.write(s)?;
                    }
                    None => self.write("32,16")?,
                }
                self.write(")")
            }
            TypeFamily::Boolean => self.write("BIT"),
            TypeFamily::Uuid => self.write("UNIQUEIDENTIFIER"),
            TypeFamily::DateTime => self.write("DATETIMEOFFSET"),
            TypeFamily::Bytes(len) => {
                self.write("VARBINARY(")?;
                match len {
                    Some(TypeDataLength::Constant(len)) => self.write(len)?,
                    Some(TypeDataLength::Maximum) => self.write("MAX")?,
                    None => self.write(8000)?,
                }
                self.write(")")
            }
        }
    }

    fn create_generated_keys(&mut self, columns: Vec<Column<'a>>) -> visitor::Result {
        self.write("DECLARE ")?;
        self.write(GENERATED_KEYS)?;
        self.write(" table")?;

        self.surround_with("(", ")", move |this| {
            let columns_len = columns.len();

            for (i, column) in columns.into_iter().enumerate() {
                this.visit_column(Column::from(column.name.into_owned()))?;
                this.write(" ")?;

                match column.type_family {
                    Some(type_family) => this.visit_type_family(type_family)?,
                    None => this.write("NVARCHAR(255)")?,
                }

                if i < (columns_len - 1) {
                    this.write(",")?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn select_generated_keys(&mut self, columns: Vec<Column<'a>>, target_table: Table<'a>) -> visitor::Result {
        let col_len = columns.len();

        let join = columns
            .iter()
            .fold(JoinData::from(target_table.alias("t")), |acc, col| {
                let left = Column::from(("t", col.name.to_string()));
                let right = Column::from(("g", col.name.to_string()));

                acc.on((left).equals(right))
            });

        self.write("SELECT ")?;

        for (i, col) in columns.into_iter().enumerate() {
            self.visit_column(Column::from(col.name.into_owned()).table("t"))?;

            if i < (col_len - 1) {
                self.write(",")?;
            }
        }

        self.write(" FROM ")?;
        self.write(GENERATED_KEYS)?;
        self.write(" AS g")?;
        self.visit_joins(vec![Join::Inner(join)])?;

        self.write(" WHERE @@ROWCOUNT > 0")?;

        Ok(())
    }

    fn visit_order_by(&mut self, direction: &str, value: Expression<'a>) -> visitor::Result {
        self.visit_expression(value)?;
        self.write(format!(" {direction}"))?;

        Ok(())
    }

    // ORDER BY CASE WHEN <value> IS NULL THEN 0 ELSE 1 END, <value> <direction>
    fn visit_order_by_nulls_first(&mut self, direction: &str, value: Expression<'a>) -> visitor::Result {
        self.surround_with("CASE WHEN ", " END", |s| {
            s.visit_expression(value.clone())?;
            s.write(" IS NULL THEN 0 ELSE 1")
        })?;
        self.write(", ")?;
        self.visit_order_by(direction, value)?;

        Ok(())
    }

    // ORDER BY CASE WHEN <value> IS NULL THEN 1 ELSE 0 END, <value> <direction>
    fn visit_order_by_nulls_last(&mut self, direction: &str, value: Expression<'a>) -> visitor::Result {
        self.surround_with("CASE WHEN ", " END", |s| {
            s.visit_expression(value.clone())?;
            s.write(" IS NULL THEN 1 ELSE 0")
        })?;
        self.write(", ")?;
        self.visit_order_by(direction, value)?;

        Ok(())
    }

    fn visit_text(&mut self, txt: Option<Cow<'a, str>>, nt: Option<NativeColumnType<'a>>) -> visitor::Result {
        self.add_parameter(Value {
            typed: ValueType::Text(txt),
            native_column_type: nt,
        });
        self.parameter_substitution()
    }
}

impl<'a> Visitor<'a> for Mssql<'a> {
    const C_BACKTICK_OPEN: &'static str = "[";
    const C_BACKTICK_CLOSE: &'static str = "]";
    const C_WILDCARD: &'static str = "%";

    fn build_template<Q>(query: Q) -> crate::Result<QueryTemplate<Value<'a>>>
    where
        Q: Into<Query<'a>>,
    {
        let mut this = Mssql {
            query_template: QueryTemplate::new(PlaceholderFormat {
                prefix: "@P",
                has_numbering: true,
            }),
            order_by_set: false,
        };

        Mssql::visit_query(&mut this, query.into())?;

        Ok(this.query_template)
    }

    fn write(&mut self, value: impl std::fmt::Display) -> visitor::Result {
        self.query_template.write_string_chunk(value.to_string());
        Ok(())
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.query_template.parameters.push(value)
    }

    fn parameter_substitution(&mut self) -> visitor::Result {
        self.query_template.write_parameter();
        Ok(())
    }

    fn visit_parameterized_row(
        &mut self,
        value: Value<'a>,
        item_prefix: impl Into<Cow<'static, str>>,
        separator: impl Into<Cow<'static, str>>,
        item_suffix: impl Into<Cow<'static, str>>,
    ) -> visitor::Result {
        self.query_template
            .write_parameter_tuple(item_prefix, separator, item_suffix);
        self.query_template.parameters.push(value);
        Ok(())
    }

    fn visit_columns(&mut self, columns: Vec<Expression<'a>>) -> visitor::Result {
        let len = columns.len();

        let columns = match columns.into_iter().exactly_one() {
            Ok(Expression {
                kind: ExpressionKind::ParameterizedRow(row),
                ..
            }) => {
                // If we have a parameterized SELECT <rows>, we want the client to dynamically
                // generate a list of rows, separated by `UNION ALL`, e.g.:
                // `SELECT 1, 2 UNION ALL SELECT 3, 4 UNION ALL SELECT 5, 6`
                self.query_template
                    .write_parameter_tuple_list("", ",", "", " UNION ALL SELECT ");
                self.query_template.parameters.push(row);
                return Ok(());
            }
            Ok(other) => Either::Left(iter::once(other)),
            Err(columns) => Either::Right(columns),
        };

        for (i, column) in columns.enumerate() {
            self.visit_expression(column)?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        Ok(())
    }

    fn visit_parameterized_text(
        &mut self,
        txt: Option<Cow<'a, str>>,
        nt: Option<NativeColumnType<'a>>,
    ) -> visitor::Result {
        match nt {
            Some(nt) => match (nt.name.as_ref(), nt.length) {
                // Tiberius encodes strings as NVARCHAR by default. This causes implicit coercions which avoids using indexes.
                // This cast ensures that VARCHAR instead.
                ("VARCHAR", length) => self.surround_with("CAST(", ")", |this| {
                    this.visit_text(txt, Some(nt))?;
                    this.write(" AS VARCHAR")?;

                    match length {
                        Some(TypeDataLength::Constant(length)) => {
                            this.write("(")?;
                            this.write(length)?;
                            this.write(")")?;
                        }
                        Some(TypeDataLength::Maximum) => {
                            this.write("(MAX)")?;
                        }
                        None => (),
                    }

                    Ok(())
                }),
                _ => self.visit_text(txt, Some(nt)),
            },
            nt => self.visit_text(txt, nt),
        }
    }

    /// A point to modify an incoming query to make it compatible with the
    /// SQL Server.
    fn compatibility_modifications(&self, query: Query<'a>) -> Query<'a> {
        match query {
            // Finding possible `(a, b) (NOT) IN (SELECT x, y ...)` comparisons,
            // and replacing them with common table expressions.
            Query::Select(select) => select
                .convert_tuple_selects_to_ctes(true, &mut 0)
                .expect_left("Top-level query was right")
                .into(),
            // Replacing the `ON CONFLICT DO NOTHING` clause with a `MERGE` statement.
            Query::Insert(insert) => match insert.on_conflict {
                Some(OnConflict::DoNothing) => Merge::try_from(*insert).unwrap().into(),
                _ => Query::Insert(insert),
            },
            _ => query,
        }
    }

    fn visit_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        match (left.kind, right.kind) {
            // we can't compare with tuples, so we'll convert it to an AND
            (ExpressionKind::Row(left), ExpressionKind::Row(right)) => {
                self.visit_multiple_tuple_comparison(left, Values::from(iter::once(right)), false)?;
            }
            (left_kind, right_kind) => {
                let (l_alias, r_alias) = (left.alias, right.alias);
                let (left_xml, right_xml) = (left_kind.is_xml_value(), right_kind.is_xml_value());

                let mut left = Expression::from(left_kind);

                if let Some(alias) = l_alias {
                    left = left.alias(alias);
                }

                let mut right = Expression::from(right_kind);

                if let Some(alias) = r_alias {
                    right = right.alias(alias);
                }

                if right_xml {
                    self.surround_with("CAST(", " AS NVARCHAR(MAX))", |x| x.visit_expression(left))?;
                } else {
                    self.visit_expression(left)?;
                }

                self.write(" = ")?;

                if left_xml {
                    self.surround_with("CAST(", " AS NVARCHAR(MAX))", |x| x.visit_expression(right))?;
                } else {
                    self.visit_expression(right)?;
                }
            }
        }

        Ok(())
    }

    fn visit_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        match (left.kind, right.kind) {
            // we can't compare with tuples, so we'll convert it to an AND
            (ExpressionKind::Row(left), ExpressionKind::Row(right)) => {
                self.visit_multiple_tuple_comparison(left, Values::from(iter::once(right)), true)?;
            }
            (left_kind, right_kind) => {
                let (l_alias, r_alias) = (left.alias, right.alias);
                let (left_xml, right_xml) = (left_kind.is_xml_value(), right_kind.is_xml_value());

                let mut left = Expression::from(left_kind);

                if let Some(alias) = l_alias {
                    left = left.alias(alias);
                }

                let mut right = Expression::from(right_kind);

                if let Some(alias) = r_alias {
                    right = right.alias(alias);
                }

                if right_xml {
                    self.surround_with("CAST(", " AS NVARCHAR(MAX))", |x| x.visit_expression(left))?;
                } else {
                    self.visit_expression(left)?;
                }

                self.write(" <> ")?;

                if left_xml {
                    self.surround_with("CAST(", " AS NVARCHAR(MAX))", |x| x.visit_expression(right))?;
                } else {
                    self.visit_expression(right)?;
                }
            }
        }

        Ok(())
    }

    fn visit_raw_value(&mut self, value: Value<'a>) -> visitor::Result {
        let res = match value.typed {
            ValueType::Int32(i) => i.map(|i| self.write(i)),
            ValueType::Int64(i) => i.map(|i| self.write(i)),
            ValueType::Float(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f32::INFINITY => self.write("'Infinity'"),
                f if f == f32::NEG_INFINITY => self.write("'-Infinity"),
                v => self.write(format!("{v:?}")),
            }),
            ValueType::Double(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f64::INFINITY => self.write("'Infinity'"),
                f if f == f64::NEG_INFINITY => self.write("'-Infinity"),
                v => self.write(format!("{v:?}")),
            }),
            ValueType::Text(t) => t.map(|t| self.write(format!("'{t}'"))),
            ValueType::Enum(e, _) => e.map(|e| self.write(e)),
            ValueType::Bytes(b) => b.map(|b| self.write(format!("0x{}", hex::encode(b)))),
            ValueType::Boolean(b) => b.map(|b| self.write(if b { 1 } else { 0 })),
            ValueType::Char(c) => c.map(|c| self.write(format!("'{c}'"))),
            ValueType::Array(_) | ValueType::EnumArray(_, _) => {
                let msg = "Arrays are not supported in T-SQL.";
                let kind = ErrorKind::conversion(msg);

                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);

                return Err(builder.build());
            }

            ValueType::Json(j) => j.map(|j| self.write(format!("'{}'", serde_json::to_string(&j).unwrap()))),

            ValueType::Numeric(r) => r.map(|r| self.write(r)),
            ValueType::Uuid(uuid) => uuid.map(|uuid| {
                let s = format!("CONVERT(uniqueidentifier, N'{}')", uuid.hyphenated());
                self.write(s)
            }),
            ValueType::DateTime(dt) => dt.map(|dt| {
                let s = format!("CONVERT(datetimeoffset, N'{}')", dt.to_rfc3339());
                self.write(s)
            }),
            ValueType::Date(date) => date.map(|date| {
                let s = format!("CONVERT(date, N'{date}')");
                self.write(s)
            }),
            ValueType::Time(time) => time.map(|time| {
                let s = format!("CONVERT(time, N'{time}')");
                self.write(s)
            }),
            // Style 3 is keep all whitespace + internal DTD processing:
            // https://docs.microsoft.com/en-us/sql/t-sql/functions/cast-and-convert-transact-sql?redirectedfrom=MSDN&view=sql-server-ver15#xml-styles
            ValueType::Xml(cow) => cow.map(|cow| self.write(format!("CONVERT(XML, N'{cow}', 3)"))),

            ValueType::Opaque(opaque) => Some(Err(
                Error::builder(ErrorKind::OpaqueAsRawValue(opaque.to_string())).build()
            )),
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> visitor::Result {
        let add_ordering = |this: &mut Self| {
            if !this.order_by_set {
                this.write(" ORDER BY ")?;
                this.visit_ordering(Ordering::new(vec![(1.raw().into(), None)]))?;
            }

            Ok::<(), crate::error::Error>(())
        };

        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                add_ordering(self)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)?;
                self.write(" ROWS FETCH NEXT ")?;
                self.visit_parameterized(limit)?;
                self.write(" ROWS ONLY")
            }
            (None, Some(offset)) if self.order_by_set || offset.typed.as_i64().map(|i| i > 0).unwrap_or(false) => {
                add_ordering(self)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)?;
                self.write(" ROWS")
            }
            (Some(limit), None) => {
                add_ordering(self)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(Value::from(0))?;
                self.write(" ROWS FETCH NEXT ")?;
                self.visit_parameterized(limit)?;
                self.write(" ROWS ONLY")
            }
            (None, _) => Ok(()),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitor::Result {
        if let Some(returning) = insert.returning.as_ref().cloned() {
            self.create_generated_keys(returning)?;
            self.write(" ")?;
        }

        self.write("INSERT")?;

        if let Some(ref table) = insert.table {
            self.write(" INTO ")?;
            self.visit_table(table.clone(), true)?;
        }

        match insert.values {
            Expression {
                kind: ExpressionKind::Parameterized(row),
                ..
            } => {
                self.write(" ")?;
                self.visit_row(Row::from(insert.columns))?;

                if let Some(ref returning) = insert.returning {
                    self.visit_returning(returning.clone())?;
                }

                self.write(" VALUES ")?;
                self.query_template.write_parameter_tuple_list("(", ",", ")", ",");
                self.query_template.parameters.push(row);
            }
            Expression {
                kind: ExpressionKind::Row(row),
                ..
            } => {
                if row.values.is_empty() {
                    if let Some(ref returning) = insert.returning {
                        self.visit_returning(returning.clone())?;
                    }

                    self.write(" DEFAULT VALUES")?;
                } else {
                    self.write(" ")?;
                    self.visit_row(Row::from(insert.columns))?;

                    if let Some(ref returning) = insert.returning {
                        self.visit_returning(returning.clone())?;
                    }

                    self.write(" VALUES ")?;
                    self.visit_row(row)?;
                }
            }
            Expression {
                kind: ExpressionKind::Values(values),
                ..
            } => {
                self.write(" ")?;
                self.visit_row(Row::from(insert.columns))?;

                if let Some(ref returning) = insert.returning {
                    self.visit_returning(returning.clone())?;
                }

                self.write(" VALUES ")?;

                let values_len = values.len();
                for (i, row) in values.into_iter().enumerate() {
                    self.visit_row(row)?;

                    if i < (values_len - 1) {
                        self.write(",")?;
                    }
                }
            }
            expr => self.surround_with("(", ")", |ref mut s| s.visit_expression(expr))?,
        }

        if let Some(returning) = insert.returning {
            let table = insert.table.unwrap();
            self.write(" ")?;
            self.select_generated_keys(returning, table)?;
        }

        if let Some(comment) = insert.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }

    // Implements `RETURNING` using the `OUTPUT` clause in SQL Server.
    fn visit_update(&mut self, update: Update<'a>) -> visitor::Result {
        if let Some(returning) = update.returning.as_ref().cloned() {
            self.create_generated_keys(returning)?;
            self.write(" ")?;
        }

        self.write("UPDATE ")?;
        self.visit_table(update.table.clone(), true)?;

        {
            self.write(" SET ")?;
            let pairs = update.columns.into_iter().zip(update.values);
            let len = pairs.len();

            for (i, (key, value)) in pairs.enumerate() {
                self.visit_column(key)?;
                self.write(" = ")?;
                self.visit_expression(value)?;

                if i < (len - 1) {
                    self.write(", ")?;
                }
            }

            if let Some(returning) = update.returning.as_ref().cloned() {
                self.visit_returning(returning)?;
            }
        }

        if let Some(conditions) = update.conditions {
            self.write(" WHERE ")?;
            self.visit_conditions(conditions)?;
        }

        if let Some(returning) = update.returning {
            let table = update.table;
            self.write(" ")?;
            self.select_generated_keys(returning, table)?;
        }

        if let Some(comment) = update.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }

    fn visit_merge(&mut self, merge: Merge<'a>) -> visitor::Result {
        if let Some(returning) = merge.returning.as_ref().cloned() {
            self.create_generated_keys(returning)?;
            self.write(" ")?;
        }

        self.write("MERGE INTO ")?;
        self.visit_table(merge.table.clone(), true)?;

        self.write(" USING ")?;

        let base_query = merge.using.base_query;
        self.surround_with("(", ")", |ref mut s| s.visit_query(base_query))?;

        self.write(" AS ")?;
        self.visit_table(merge.using.as_table, false)?;

        self.write(" ")?;
        self.visit_row(Row::from(merge.using.columns))?;
        self.write(" ON ")?;
        self.visit_conditions(merge.using.on_conditions)?;

        if let Some(query) = merge.when_not_matched {
            self.write(" WHEN NOT MATCHED THEN ")?;
            self.visit_query(query)?;
        }

        if let Some(columns) = merge.returning {
            self.visit_returning(columns.clone())?;
            self.write("; ")?;
            self.select_generated_keys(columns, merge.table)?;
        } else {
            self.write(";")?;
        }

        Ok(())
    }

    fn visit_upsert(&mut self, _update: crate::ast::Update<'a>) -> visitor::Result {
        unimplemented!("Upsert not supported for the underlying database.")
    }

    fn visit_aggregate_to_string(&mut self, value: crate::ast::Expression<'a>) -> visitor::Result {
        self.write("STRING_AGG")?;
        self.surround_with("(", ")", |ref mut se| {
            se.visit_expression(value)?;
            se.write(",")?;
            se.write("\",\"")
        })
    }

    // MSSQL doesn't support tuples, we do AND/OR.
    fn visit_multiple_tuple_comparison(&mut self, left: Row<'a>, right: Values<'a>, negate: bool) -> visitor::Result {
        let row_len = left.len();
        let values_len = right.len();

        if negate {
            self.write("NOT ")?;
        }

        self.surround_with("(", ")", |this| {
            for (i, row) in right.into_iter().enumerate() {
                this.surround_with("(", ")", |se| {
                    let row_and_vals = left.values.clone().into_iter().zip(row.values.into_iter());

                    for (j, (expr, val)) in row_and_vals.enumerate() {
                        se.visit_expression(expr)?;
                        se.write(" = ")?;
                        se.visit_expression(val)?;

                        if j < row_len - 1 {
                            se.write(" AND ")?;
                        }
                    }

                    Ok(())
                })?;

                if i < values_len - 1 {
                    this.write(" OR ")?;
                }
            }

            Ok(())
        })
    }

    fn visit_ordering(&mut self, ordering: Ordering<'a>) -> visitor::Result {
        let len = ordering.0.len();

        for (i, (value, ordering)) in ordering.0.into_iter().enumerate() {
            match ordering {
                Some(Order::Asc) => {
                    self.visit_order_by("ASC", value)?;
                }
                Some(Order::Desc) => {
                    self.visit_order_by("DESC", value)?;
                }
                Some(Order::AscNullsFirst) => {
                    self.visit_order_by_nulls_first("ASC", value)?;
                }
                Some(Order::AscNullsLast) => {
                    self.visit_order_by_nulls_last("ASC", value)?;
                }
                Some(Order::DescNullsFirst) => {
                    self.visit_order_by_nulls_first("DESC", value)?;
                }
                Some(Order::DescNullsLast) => {
                    self.visit_order_by_nulls_last("DESC", value)?;
                }
                None => {
                    self.visit_expression(value)?;
                }
            };

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        self.order_by_set = true;

        Ok(())
    }

    fn visit_average(&mut self, avg: Average<'a>) -> visitor::Result {
        self.write("AVG")?;

        // SQL Server will average as an integer, so average of 0 an 1 would be
        // 0, if we don't convert the value to a decimal first.
        self.surround_with("(", ")", |ref mut s| {
            s.write("CONVERT")?;

            s.surround_with("(", ")", |ref mut s| {
                s.write("DECIMAL(32,16),")?;
                s.visit_column(avg.column)
            })
        })?;

        Ok(())
    }

    fn visit_json_extract(&mut self, _json_extract: JsonExtract<'a>) -> visitor::Result {
        unimplemented!("JSON filtering is not yet supported on MSSQL")
    }

    fn visit_json_array_contains(
        &mut self,
        _left: Expression<'a>,
        _right: Expression<'a>,
        _not: bool,
    ) -> visitor::Result {
        unimplemented!("JSON filtering is not yet supported on MSSQL")
    }

    fn visit_json_type_equals(&mut self, _left: Expression<'a>, _json_type: JsonType, _not: bool) -> visitor::Result {
        unimplemented!("JSON_TYPE is not yet supported on MSSQL")
    }

    fn visit_json_unquote(&mut self, _json_unquote: JsonUnquote<'a>) -> visitor::Result {
        unimplemented!("JSON filtering is not yet supported on MSSQL")
    }

    fn visit_json_array_agg(&mut self, _array_agg: JsonArrayAgg<'a>) -> visitor::Result {
        unimplemented!("JSON_AGG is not yet supported on MSSQL")
    }

    fn visit_json_build_object(&mut self, _build_obj: JsonBuildObject<'a>) -> visitor::Result {
        unimplemented!("JSON_BUILD_OBJECT is not yet supported on MSSQL")
    }

    fn visit_text_search(&mut self, _text_search: crate::prelude::TextSearch<'a>) -> visitor::Result {
        unimplemented!("Full-text search is not yet supported on MSSQL")
    }

    fn visit_matches(&mut self, _left: Expression<'a>, _right: Expression<'a>, _not: bool) -> visitor::Result {
        unimplemented!("Full-text search is not yet supported on MSSQL")
    }

    fn visit_text_search_relevance(
        &mut self,
        _text_search_relevance: crate::prelude::TextSearchRelevance<'a>,
    ) -> visitor::Result {
        unimplemented!("Full-text search is not yet supported on MSSQL")
    }

    fn visit_json_extract_last_array_item(
        &mut self,
        _extract: crate::prelude::JsonExtractLastArrayElem<'a>,
    ) -> visitor::Result {
        unimplemented!("JSON filtering is not yet supported on MSSQL")
    }

    fn visit_json_extract_first_array_item(
        &mut self,
        _extract: crate::prelude::JsonExtractFirstArrayElem<'a>,
    ) -> visitor::Result {
        unimplemented!("JSON filtering is not yet supported on MSSQL")
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::*,
        visitor::{Mssql, Visitor},
    };
    use indoc::indoc;

    fn expected_values<'a, T>(sql: &'static str, params: Vec<T>) -> (String, Vec<Value<'a>>)
    where
        T: Into<Value<'a>>,
    {
        (String::from(sql), params.into_iter().map(|p| p.into()).collect())
    }

    fn default_params(mut additional: Vec<Value<'_>>) -> Vec<Value<'_>> {
        let mut result = Vec::new();

        for param in additional.drain(0..) {
            result.push(param)
        }

        result
    }

    #[test]
    fn test_select_1() {
        let expected = expected_values("SELECT @P1", vec![1]);

        let query = Select::default().value(1);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_value() {
        let expected = expected_values("SELECT @P1 AS [test]", vec![1]);

        let query = Select::default().value(val!(1).alias("test"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_null() {
        let expected_sql = "SELECT @P1 AS [test]";
        let query = Select::default().value(val!(ValueType::Int32(None)).alias("test"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::null_int32()], params);
    }

    #[test]
    fn test_select_star_from() {
        let expected_sql = "SELECT [musti].* FROM [musti]";
        let query = Select::from_table("musti");
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_in_values() {
        let expected_sql =
            "SELECT [test].* FROM [test] WHERE (([id1] = @P1 AND [id2] = @P2) OR ([id1] = @P3 AND [id2] = @P4))";

        let query = Select::from_table("test")
            .so_that(Row::from((col!("id1"), col!("id2"))).in_selection(values!((1, 2), (3, 4))));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![Value::int32(1), Value::int32(2), Value::int32(3), Value::int32(4),],
            params
        );
    }

    #[test]
    fn test_not_in_values() {
        let expected_sql =
            "SELECT [test].* FROM [test] WHERE NOT (([id1] = @P1 AND [id2] = @P2) OR ([id1] = @P3 AND [id2] = @P4))";

        let query = Select::from_table("test")
            .so_that(Row::from((col!("id1"), col!("id2"))).not_in_selection(values!((1, 2), (3, 4))));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![Value::int32(1), Value::int32(2), Value::int32(3), Value::int32(4),],
            params
        );
    }

    #[test]
    fn test_in_values_singular() {
        let mut cols = Row::new();
        cols.push(Column::from("id1"));

        let mut vals = Values::new(vec![]);

        {
            let mut row1 = Row::new();
            row1.push(1);

            let mut row2 = Row::new();
            row2.push(2);

            vals.push(row1);
            vals.push(row2);
        }

        let query = Select::from_table("test").so_that(cols.in_selection(vals));
        let (sql, params) = Mssql::build(query).unwrap();
        let expected_sql = "SELECT [test].* FROM [test] WHERE [id1] IN (@P1,@P2)";

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::int32(1), Value::int32(2),], params)
    }

    #[test]
    fn test_select_order_by() {
        let expected_sql = "SELECT [musti].* FROM [musti] ORDER BY [foo], [baz] ASC, [bar] DESC";
        let query = Select::from_table("musti")
            .order_by("foo")
            .order_by("baz".ascend())
            .order_by("bar".descend());
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_fields_from() {
        let expected_sql = "SELECT [paw], [nose] FROM [cat].[musti]";
        let query = Select::from_table(("cat", "musti")).column("paw").column("nose");
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_where_equals() {
        let expected = expected_values("SELECT [naukio].* FROM [naukio] WHERE [word] = @P1", vec!["meow"]);

        let query = Select::from_table("naukio").so_that("word".equals("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_like() {
        let expected = expected_values("SELECT [naukio].* FROM [naukio] WHERE [word] LIKE @P1", vec!["%meow%"]);

        let query = Select::from_table("naukio").so_that("word".like("%meow%"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_like() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [word] NOT LIKE @P1",
            vec!["%meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".not_like("%meow%"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_begins_with() {
        let expected = expected_values("SELECT [naukio].* FROM [naukio] WHERE [word] LIKE @P1", vec!["%meow"]);

        let query = Select::from_table("naukio").so_that("word".like("%meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_begins_with() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [word] NOT LIKE @P1",
            vec!["%meow"],
        );

        let query = Select::from_table("naukio").so_that("word".not_like("%meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_ends_into() {
        let expected = expected_values("SELECT [naukio].* FROM [naukio] WHERE [word] LIKE @P1", vec!["meow%"]);

        let query = Select::from_table("naukio").so_that("word".like("meow%"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_ends_into() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [word] NOT LIKE @P1",
            vec!["meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".not_like("meow%"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn equality_with_a_xml_value() {
        let expected = expected_values(
            r#"SELECT [users].* FROM [users] WHERE CAST([xmlField] AS NVARCHAR(MAX)) = @P1"#,
            vec![Value::xml("<cat>meow</cat>")],
        );

        let query = Select::from_table("users").so_that(Column::from("xmlField").equals(Value::xml("<cat>meow</cat>")));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn equality_with_a_lhs_xml_value() {
        let expected = expected_values(
            r#"SELECT [users].* FROM [users] WHERE @P1 = CAST([xmlField] AS NVARCHAR(MAX))"#,
            vec![Value::xml("<cat>meow</cat>")],
        );

        let value_expr: Expression = Value::xml("<cat>meow</cat>").into();
        let query = Select::from_table("users").so_that(value_expr.equals(Column::from("xmlField")));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_xml_value() {
        let expected = expected_values(
            r#"SELECT [users].* FROM [users] WHERE CAST([xmlField] AS NVARCHAR(MAX)) <> @P1"#,
            vec![Value::xml("<cat>meow</cat>")],
        );

        let query =
            Select::from_table("users").so_that(Column::from("xmlField").not_equals(Value::xml("<cat>meow</cat>")));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_lhs_xml_value() {
        let expected = expected_values(
            r#"SELECT [users].* FROM [users] WHERE @P1 <> CAST([xmlField] AS NVARCHAR(MAX))"#,
            vec![Value::xml("<cat>meow</cat>")],
        );

        let value_expr: Expression = Value::xml("<cat>meow</cat>").into();
        let query = Select::from_table("users").so_that(value_expr.not_equals(Column::from("xmlField")));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_and() {
        let expected_sql = "SELECT [naukio].* FROM [naukio] WHERE ([word] = @P1 AND [age] < @P2 AND [paw] = @P3)";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = "word".equals("meow").and("age".less_than(10)).and("paw".equals("warm"));
        let query = Select::from_table("naukio").so_that(conditions);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_and_different_execution_order() {
        let expected_sql = "SELECT [naukio].* FROM [naukio] WHERE ([word] = @P1 AND ([age] < @P2 AND [paw] = @P3))";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = "word".equals("meow").and("age".less_than(10).and("paw".equals("warm")));
        let query = Select::from_table("naukio").so_that(conditions);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_or() {
        let expected_sql = "SELECT [naukio].* FROM [naukio] WHERE (([word] = @P1 OR [age] < @P2) AND [paw] = @P3)";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = "word".equals("meow").or("age".less_than(10)).and("paw".equals("warm"));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_negation() {
        let expected_sql =
            "SELECT [naukio].* FROM [naukio] WHERE (NOT (([word] = @P1 OR [age] < @P2) AND [paw] = @P3))";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = "word"
            .equals("meow")
            .or("age".less_than(10))
            .and("paw".equals("warm"))
            .not();

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_with_raw_condition_tree() {
        let expected_sql =
            "SELECT [naukio].* FROM [naukio] WHERE (NOT (([word] = @P1 OR [age] < @P2) AND [paw] = @P3))";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = ConditionTree::not("word".equals("meow").or("age".less_than(10)).and("paw".equals("warm")));
        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_simple_inner_join() {
        let expected_sql = "SELECT [users].* FROM [users] INNER JOIN [posts] ON [users].[id] = [posts].[user_id]";

        let query = Select::from_table("users")
            .inner_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_inner_join() {
        let expected_sql = "SELECT [users].* FROM [users] INNER JOIN [posts] ON ([users].[id] = [posts].[user_id] AND [posts].[published] = @P1)";

        let query = Select::from_table("users").inner_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[test]
    fn test_simple_left_join() {
        let expected_sql = "SELECT [users].* FROM [users] LEFT JOIN [posts] ON [users].[id] = [posts].[user_id]";

        let query = Select::from_table("users")
            .left_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_left_join() {
        let expected_sql = "SELECT [users].* FROM [users] LEFT JOIN [posts] ON ([users].[id] = [posts].[user_id] AND [posts].[published] = @P1)";

        let query = Select::from_table("users").left_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[test]
    fn test_column_aliasing() {
        let expected_sql = "SELECT [bar] AS [foo] FROM [meow]";
        let query = Select::from_table("meow").column(Column::new("bar").alias("foo"));
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_limit_with_no_offset() {
        let expected_sql = "SELECT [foo] FROM [bar] ORDER BY [id] OFFSET @P1 ROWS FETCH NEXT @P2 ROWS ONLY";
        let query = Select::from_table("bar").column("foo").order_by("id").limit(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::int32(0), Value::int64(10)], params);
    }

    #[test]
    fn test_offset_no_limit() {
        let expected_sql = "SELECT [foo] FROM [bar] ORDER BY [id] OFFSET @P1 ROWS";
        let query = Select::from_table("bar").column("foo").order_by("id").offset(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        // NOTE: Offsets are unsigned, so they only fit in i64, not i32
        assert_eq!(vec![Value::int64(10)], params);
    }

    #[test]
    fn test_limit_with_offset() {
        let expected_sql = "SELECT [foo] FROM [bar] ORDER BY [id] OFFSET @P1 ROWS FETCH NEXT @P2 ROWS ONLY";
        let query = Select::from_table("bar")
            .column("foo")
            .order_by("id")
            .limit(9)
            .offset(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        // NOTE: offset and limits cannot be negative, they are u32, so they fit in i64
        assert_eq!(vec![Value::int64(10), Value::int64(9)], params);
    }

    #[test]
    fn test_limit_with_offset_no_given_order() {
        let expected_sql = "SELECT [foo] FROM [bar] ORDER BY 1 OFFSET @P1 ROWS FETCH NEXT @P2 ROWS ONLY";
        let query = Select::from_table("bar").column("foo").limit(9).offset(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::int64(10), Value::int64(9)], params);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) = Mssql::build(Select::default().value(ValueType::Text(None).raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = Mssql::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_real() {
        let (sql, params) = Mssql::build(Select::default().value(1.3f64.raw())).unwrap();
        assert_eq!("SELECT 1.3", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text() {
        let (sql, params) = Mssql::build(Select::default().value("foo".raw())).unwrap();
        assert_eq!("SELECT 'foo'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_bytes() {
        let (sql, params) = Mssql::build(Select::default().value(Value::bytes(vec![1, 2, 3]).raw())).unwrap();

        assert_eq!("SELECT 0x010203", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = Mssql::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());

        let (sql, params) = Mssql::build(Select::default().value(false.raw())).unwrap();
        assert_eq!("SELECT 0", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_char() {
        let (sql, params) = Mssql::build(Select::default().value(Value::character('a').raw())).unwrap();
        assert_eq!("SELECT 'a'", sql);
        assert!(params.is_empty());
    }

    #[test]

    fn test_raw_json() {
        let (sql, params) = Mssql::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw())).unwrap();
        assert_eq!("SELECT '{\"foo\":\"bar\"}'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let (sql, params) = Mssql::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(
            format!("SELECT CONVERT(uniqueidentifier, N'{}')", uuid.hyphenated()),
            sql
        );

        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_datetime() {
        let dt = chrono::Utc::now();
        let (sql, params) = Mssql::build(Select::default().value(dt.raw())).unwrap();

        assert_eq!(format!("SELECT CONVERT(datetimeoffset, N'{}')", dt.to_rfc3339(),), sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_single_insert() {
        let insert = Insert::single_into("foo").value("bar", "lol").value("wtf", "meow");
        let (sql, params) = Mssql::build(insert).unwrap();

        assert_eq!("INSERT INTO [foo] ([bar],[wtf]) VALUES (@P1,@P2)", sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_default() {
        let insert = Insert::single_into("foo");
        let (sql, params) = Mssql::build(insert).unwrap();

        assert_eq!("INSERT INTO [foo] DEFAULT VALUES", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "mssql")]
    fn test_returning_insert() {
        let insert = Insert::single_into("foo").value("bar", "lol");
        let (sql, params) = Mssql::build(Insert::from(insert).returning(vec!["bar"])).unwrap();

        assert_eq!(
            "DECLARE @generated_keys table([bar] NVARCHAR(255)) INSERT INTO [foo] ([bar]) OUTPUT [Inserted].[bar] INTO @generated_keys VALUES (@P1) SELECT [t].[bar] FROM @generated_keys AS g INNER JOIN [foo] AS [t] ON [t].[bar] = [g].[bar] WHERE @@ROWCOUNT > 0",
            sql
        );

        assert_eq!(vec![Value::from("lol")], params);
    }

    #[test]
    fn test_multi_insert() {
        let insert = Insert::multi_into("foo", vec!["bar", "wtf"])
            .values(vec!["lol", "meow"])
            .values(vec!["omg", "hey"]);

        let (sql, params) = Mssql::build(insert).unwrap();

        assert_eq!("INSERT INTO [foo] ([bar],[wtf]) VALUES (@P1,@P2),(@P3,@P4)", sql);

        assert_eq!(
            vec![
                Value::from("lol"),
                Value::from("meow"),
                Value::from("omg"),
                Value::from("hey")
            ],
            params
        );
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_single_unique() {
        let table = Table::from("foo").add_unique_index("bar");

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "bar"), "lol")
            .value(("foo", "wtf"), "meow")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON [dual].[bar] = [foo].[bar]
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_single_unique_with_default() {
        let unique_column = Column::from("bar").default("purr");
        let table = Table::from("foo").add_unique_index(unique_column);

        let insert: Insert<'_> = Insert::single_into(table).value(("foo", "wtf"), "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON [foo].[bar] = @P2
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    #[cfg(feature = "mssql")]
    fn test_single_insert_conflict_with_returning_clause() {
        let table = Table::from("foo").add_unique_index("bar");

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "bar"), "lol")
            .value(("foo", "wtf"), "meow")
            .into();

        let insert = insert
            .on_conflict(OnConflict::DoNothing)
            .returning(vec![("foo", "bar"), ("foo", "wtf")]);

        let (sql, params) = Mssql::build(insert).unwrap();

        let expected_sql = indoc!(
            "
            DECLARE @generated_keys table([bar] NVARCHAR(255),[wtf] NVARCHAR(255))
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON [dual].[bar] = [foo].[bar]
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf])
            OUTPUT [Inserted].[bar],[Inserted].[wtf] INTO @generated_keys;
            SELECT [t].[bar],[t].[wtf] FROM @generated_keys AS g
            INNER JOIN [foo] AS [t]
            ON ([t].[bar] = [g].[bar] AND [t].[wtf] = [g].[wtf])
            WHERE @@ROWCOUNT > 0
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_two_uniques() {
        let table = Table::from("foo").add_unique_index("bar").add_unique_index("wtf");

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "bar"), "lol")
            .value(("foo", "wtf"), "meow")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON ([dual].[bar] = [foo].[bar] OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_two_uniques_with_default() {
        let unique_column = Column::from("bar").default("purr");

        let table = Table::from("foo")
            .add_unique_index(unique_column)
            .add_unique_index("wtf");

        let insert: Insert<'_> = Insert::single_into(table).value(("foo", "wtf"), "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON ([foo].[bar] = @P2 OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    fn generated_unique_defaults_should_not_be_part_of_the_join_when_value_is_not_provided() {
        let unique_column = Column::from("bar").default("purr");
        let default_column = Column::from("lol").default(DefaultValue::Generated);

        let table = Table::from("foo")
            .add_unique_index(unique_column)
            .add_unique_index(default_column)
            .add_unique_index("wtf");

        let insert: Insert<'_> = Insert::single_into(table).value(("foo", "wtf"), "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON ([foo].[bar] = @P2 OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    fn with_generated_unique_defaults_the_value_should_be_part_of_the_join() {
        let unique_column = Column::from("bar").default("purr");
        let default_column = Column::from("lol").default(DefaultValue::Generated);

        let table = Table::from("foo")
            .add_unique_index(unique_column)
            .add_unique_index(default_column)
            .add_unique_index("wtf");

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "wtf"), "meow")
            .value(("foo", "lol"), "hiss")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf], @P2 AS [lol]) AS [dual] ([wtf],[lol])
            ON ([foo].[bar] = @P3 OR [dual].[lol] = [foo].[lol] OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf],[lol]) VALUES ([dual].[wtf],[dual].[lol]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);

        assert_eq!(
            vec![Value::from("meow"), Value::from("hiss"), Value::from("purr")],
            params
        );
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_compound_unique() {
        let table = Table::from("foo").add_unique_index(vec!["bar", "wtf"]);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "bar"), "lol")
            .value(("foo", "wtf"), "meow")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON ([dual].[bar] = [foo].[bar] AND [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_compound_unique_with_default() {
        let bar = Column::from("bar").default("purr");
        let wtf = Column::from("wtf");

        let table = Table::from("foo").add_unique_index(vec![bar, wtf]);
        let insert: Insert<'_> = Insert::single_into(table).value(("foo", "wtf"), "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON ([foo].[bar] = @P2 AND [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    fn one_generated_value_in_compound_unique_removes_the_whole_index_from_the_join() {
        let bar = Column::from("bar").default("purr");
        let wtf = Column::from("wtf");

        let omg = Column::from("omg").default(DefaultValue::Generated);
        let lol = Column::from("lol");

        let table = Table::from("foo")
            .add_unique_index(vec![bar, wtf])
            .add_unique_index(vec![omg, lol]);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "wtf"), "meow")
            .value(("foo", "lol"), "hiss")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf], @P2 AS [lol]) AS [dual] ([wtf],[lol])
            ON (([foo].[bar] = @P3 AND [dual].[wtf] = [foo].[wtf]) OR (1=0 AND [dual].[lol] = [foo].[lol]))
            WHEN NOT MATCHED THEN
            INSERT ([wtf],[lol]) VALUES ([dual].[wtf],[dual].[lol]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(
            vec![Value::from("meow"), Value::from("hiss"), Value::from("purr")],
            params
        );
    }

    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT [bar] FROM [test]";
        let query = Select::from_table("test").column(Column::new("bar")).distinct();
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_distinct_with_subquery() {
        let expected_sql = "SELECT DISTINCT (SELECT @P1 FROM [test2]), [bar] FROM [test]";
        let query = Select::from_table("test")
            .value(Select::from_table("test2").value(val!(1)))
            .column(Column::new("bar"))
            .distinct();

        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_from() {
        let expected_sql = "SELECT [foo].*, [bar].[a] FROM [foo], (SELECT [a] FROM [baz]) AS [bar]";
        let query = Select::default()
            .and_from("foo")
            .and_from(Table::from(Select::from_table("baz").column("a")).alias("bar"))
            .value(Table::from("foo").asterisk())
            .column(("bar", "a"));

        let (sql, _) = Mssql::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_comment_insert() {
        let expected_sql = "INSERT INTO [users] DEFAULT VALUES /* trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2' */";
        let query = Insert::single_into("users");
        let insert =
            Insert::from(query).comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");

        let (sql, _) = Mssql::build(insert).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_cte_conversion_top_level_in() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE [A].[x] IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])"#
        )
        .replace('\n', " ");

        let inner = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let row = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);
        let query = Select::from_table("A").so_that(row.in_selection(inner));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::int32(1), Value::int32(2)], params);
    }

    #[test]
    fn test_cte_conversion_top_level_not_in() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE [A].[x] NOT IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])"#
        )
        .replace('\n', " ");

        let inner = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let row = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);
        let query = Select::from_table("A").so_that(row.not_in_selection(inner));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::int32(1), Value::int32(2)], params);
    }

    #[test]
    fn test_cte_conversion_in_a_tree_top_level() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE ([A].[y] = @P3
            AND [A].[z] = @P4
            AND [A].[x] IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y]))"#
        )
        .replace('\n', " ");

        let inner = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let row = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);

        let query = Select::from_table("A")
            .so_that(("A", "y").equals("bar"))
            .and_where(("A", "z").equals("foo"))
            .and_where(row.in_selection(inner));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);

        assert_eq!(
            vec![Value::int32(1), Value::int32(2), Value::text("bar"), Value::text("foo")],
            params
        );
    }

    #[test]
    fn test_cte_conversion_in_a_tree_nested() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE ([A].[y] = @P3 OR ([A].[z] = @P4 AND [A].[x] IN
            (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])))"#
        )
        .replace('\n', " ");

        let inner = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let row = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);

        let cond = ("A", "y")
            .equals("bar")
            .or(("A", "z").equals("foo").and(row.in_selection(inner)));

        let query = Select::from_table("A").so_that(cond);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);

        assert_eq!(
            vec![Value::int32(1), Value::int32(2), Value::text("bar"), Value::text("foo")],
            params
        );
    }

    #[test]
    fn test_multiple_cte_conversions_in_the_ast() {
        let expected_sql = indoc!(
            r#"WITH
            [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b]),
            [cte_1] AS (SELECT @P3 AS [c], @P4 AS [d])
            SELECT [A].* FROM [A]
            WHERE ([A].[x] IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])
            AND [A].[u] NOT IN (SELECT [c] FROM [cte_1] WHERE [d] = [A].[z]))"#
        )
        .replace('\n', " ");

        let cte_0 = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let cte_1 = Select::default().value(val!(3).alias("c")).value(val!(4).alias("d"));
        let row_0 = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);
        let row_1 = Row::from(vec![col!(("A", "u")), col!(("A", "z"))]);

        let query = Select::from_table("A")
            .so_that(row_0.in_selection(cte_0))
            .and_where(row_1.not_in_selection(cte_1));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);

        assert_eq!(
            vec![Value::int32(1), Value::int32(2), Value::int32(3), Value::int32(4)],
            params
        );
    }

    #[test]
    fn test_default_insert() {
        let insert = Insert::single_into("foo")
            .value("foo", "bar")
            .value("baz", default_value());

        let (sql, _) = Mssql::build(insert).unwrap();

        assert_eq!("INSERT INTO [foo] ([foo],[baz]) VALUES (@P1,DEFAULT)", sql);
    }

    #[test]
    fn join_is_inserted_positionally() {
        let joined_table = Table::from("User").left_join(
            "Post"
                .alias("p")
                .on(("p", "userId").equals(Column::from(("User", "id")))),
        );
        let q = Select::from_table(joined_table).and_from("Toto");
        let (sql, _) = Mssql::build(q).unwrap();

        assert_eq!(
            "SELECT [User].*, [Toto].* FROM [User] LEFT JOIN [Post] AS [p] ON [p].[userId] = [User].[id], [Toto]",
            sql
        );
    }
}
