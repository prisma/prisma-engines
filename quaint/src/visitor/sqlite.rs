use crate::{
    ast::*,
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};

use crate::visitor::query_writer::QueryWriter;
use query_template::{PlaceholderFormat, QueryTemplate};
use std::{borrow::Cow, fmt};

/// A visitor to generate queries for the SQLite database.
///
/// The returned parameter values implement the `ToSql` trait from rusqlite and
/// can be used directly with the database.
pub struct Sqlite<'a> {
    query_template: QueryTemplate<Value<'a>>,
}

impl<'a> Sqlite<'a> {
    /// Expression that evaluates to the SQLite version.
    pub const fn version_expr() -> &'static str {
        "sqlite_version()"
    }

    fn returning(&mut self, returning: Option<Vec<Column<'a>>>) -> visitor::Result {
        if let Some(returning) = returning
            && !returning.is_empty()
        {
            let values_len = returning.len();
            self.write(" RETURNING ")?;

            for (i, column) in returning.into_iter().enumerate() {
                // Workaround for SQLite parsing bug
                // https://sqlite.org/forum/info/6c141f151fa5c444db257eb4d95c302b70bfe5515901cf987e83ed8ebd434c49?t=h
                self.surround_with_backticks(&column.name)?;
                self.write(" AS ")?;
                self.surround_with_backticks(&column.name)?;
                if i < (values_len - 1) {
                    self.write(", ")?;
                }
            }
        }
        Ok(())
    }
}

impl<'a> Sqlite<'a> {
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
}

impl<'a> Visitor<'a> for Sqlite<'a> {
    const C_BACKTICK_OPEN: &'static str = "`";
    const C_BACKTICK_CLOSE: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    fn build_template<Q>(query: Q) -> crate::Result<QueryTemplate<Value<'a>>>
    where
        Q: Into<Query<'a>>,
    {
        let mut this = Sqlite {
            query_template: QueryTemplate::new(PlaceholderFormat {
                prefix: "?",
                has_numbering: false,
            }),
        };

        Sqlite::visit_query(&mut this, query.into())?;

        Ok(this.query_template)
    }

    fn write(&mut self, value: impl fmt::Display) -> visitor::Result {
        self.query_template.write_string_chunk(value.to_string());
        Ok(())
    }

    fn visit_raw_value(&mut self, value: Value<'a>) -> visitor::Result {
        let res = match &value.typed {
            ValueType::Int32(i) => i.map(|i| self.write(i)),
            ValueType::Int64(i) => i.map(|i| self.write(i)),
            ValueType::Text(t) => t.as_ref().map(|t| self.write(format!("'{t}'"))),
            ValueType::Enum(e, _) => e.as_ref().map(|e| self.write(e)),
            ValueType::Bytes(b) => b.as_ref().map(|b| self.write(format!("x'{}'", hex::encode(b)))),
            ValueType::Boolean(b) => b.map(|b| self.write(b)),
            ValueType::Char(c) => c.map(|c| self.write(format!("'{c}'"))),
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
            ValueType::Array(_) | ValueType::EnumArray(_, _) => {
                let msg = "Arrays are not supported in SQLite.";
                let kind = ErrorKind::conversion(msg);

                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);

                return Err(builder.build());
            }

            ValueType::Json(j) => match j {
                Some(j) => {
                    let s = serde_json::to_string(j)?;
                    Some(self.write(format!("'{s}'")))
                }
                None => None,
            },

            ValueType::Numeric(r) => r.as_ref().map(|r| self.write(r)),
            ValueType::Uuid(uuid) => uuid.map(|uuid| self.write(format!("'{}'", uuid.hyphenated()))),
            ValueType::DateTime(dt) => dt.map(|dt| self.write(format!("'{}'", dt.to_rfc3339(),))),
            ValueType::Date(date) => date.map(|date| self.write(format!("'{date}'"))),
            ValueType::Time(time) => time.map(|time| self.write(format!("'{time}'"))),
            ValueType::Xml(cow) => cow.as_ref().map(|cow| self.write(format!("'{cow}'"))),

            ValueType::Opaque(opaque) => Some(Err(
                Error::builder(ErrorKind::OpaqueAsRawValue(opaque.to_string())).build()
            )),
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitor::Result {
        match insert.on_conflict {
            Some(OnConflict::DoNothing) => self.write("INSERT OR IGNORE")?,
            _ => self.write("INSERT")?,
        };

        if let Some(table) = insert.table {
            self.write(" INTO ")?;
            self.visit_table(table, true)?;
        }

        match insert.values {
            Expression {
                kind: ExpressionKind::Parameterized(row),
                ..
            } => {
                let columns = insert.columns.len();

                self.write(" (")?;
                for (i, c) in insert.columns.into_iter().enumerate() {
                    self.visit_column(c.name.into_owned().into())?;

                    if i < (columns - 1) {
                        self.write(", ")?;
                    }
                }

                self.write(")")?;
                self.write(" VALUES ")?;
                self.query_template.write_parameter_tuple_list("(", ",", ")", ",");
                self.query_template.parameters.push(row);
            }
            Expression {
                kind: ExpressionKind::Row(row),
                ..
            } => {
                if row.values.is_empty() {
                    self.write(" DEFAULT VALUES")?;
                } else {
                    let columns = insert.columns.len();

                    self.write(" (")?;
                    for (i, c) in insert.columns.into_iter().enumerate() {
                        self.visit_column(c.name.into_owned().into())?;

                        if i < (columns - 1) {
                            self.write(", ")?;
                        }
                    }

                    self.write(")")?;
                    self.write(" VALUES ")?;
                    self.visit_row(row)?;
                }
            }
            Expression {
                kind: ExpressionKind::Values(values),
                ..
            } => {
                let columns = insert.columns.len();

                self.write(" (")?;
                for (i, c) in insert.columns.into_iter().enumerate() {
                    self.visit_column(c.name.into_owned().into())?;

                    if i < (columns - 1) {
                        self.write(", ")?;
                    }
                }
                self.write(")")?;

                self.write(" VALUES ")?;
                let values_len = values.len();

                for (i, row) in values.into_iter().enumerate() {
                    self.visit_row(row)?;

                    if i < (values_len - 1) {
                        self.write(", ")?;
                    }
                }
            }
            expr => self.visit_expression(expr)?,
        }

        if let Some(OnConflict::Update(update, constraints)) = insert.on_conflict {
            self.write(" ON CONFLICT ")?;
            self.columns_to_bracket_list(constraints)?;
            self.write(" DO ")?;

            self.visit_upsert(update)?;
        }

        self.returning(insert.returning)?;

        if let Some(comment) = insert.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.query_template.parameters.push(value);
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

    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> visitor::Result {
        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (None, Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(Value::from(-1))?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (Some(limit), None) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)
            }
            (None, None) => Ok(()),
        }
    }

    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> visitor::Result {
        self.write("GROUP_CONCAT")?;
        self.surround_with("(", ")", |ref mut s| s.visit_expression(value))
    }

    fn visit_values(&mut self, values: Values<'a>) -> visitor::Result {
        self.surround_with("(VALUES ", ")", |ref mut s| {
            let len = values.len();
            for (i, row) in values.into_iter().enumerate() {
                s.visit_row(row)?;

                if i < (len - 1) {
                    s.write(",")?;
                }
            }
            Ok(())
        })
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_extract(&mut self, json_extract: JsonExtract<'a>) -> visitor::Result {
        self.visit_expression(*json_extract.column)?;

        if json_extract.extract_as_string {
            self.write("->>")?;
        } else {
            self.write("->")?;
        }

        match json_extract.path {
            JsonPath::Array(_) => panic!("JSON path array notation is not supported for SQlite"),
            JsonPath::String(path) => self.visit_parameterized(Value::text(path))?,
        }

        Ok(())
    }

    fn visit_json_array_contains(
        &mut self,
        _left: Expression<'a>,
        _right: Expression<'a>,
        _not: bool,
    ) -> visitor::Result {
        unimplemented!("JSON contains is not supported on SQLite")
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_type_equals(&mut self, left: Expression<'a>, json_type: JsonType<'a>, not: bool) -> visitor::Result {
        self.write("(")?;
        self.write("JSON_TYPE")?;
        self.surround_with("(", ")", |s| s.visit_expression(left.clone()))?;

        if not {
            self.write(" != ")?;
        } else {
            self.write(" = ")?;
        }

        match json_type {
            JsonType::Array => self.visit_expression(Expression::from(Value::text("array")))?,
            JsonType::Boolean => {
                self.visit_expression(Expression::from(Value::text("true")))?;
                self.write(" OR JSON_TYPE")?;
                self.surround_with("(", ")", |s| s.visit_expression(left))?;
                self.write(" = ")?;
                self.visit_expression(Expression::from(Value::text("false")))?;
            }
            JsonType::Number => {
                self.visit_expression(Expression::from(Value::text("integer")))?;
                self.write(" OR JSON_TYPE")?;
                self.surround_with("(", ")", |s| s.visit_expression(left))?;
                self.write(" = ")?;
                self.visit_expression(Expression::from(Value::text("real")))?;
            }
            JsonType::Object => self.visit_expression(Expression::from(Value::text("object")))?,
            JsonType::String => self.visit_expression(Expression::from(Value::text("text")))?,
            JsonType::Null => self.visit_expression(Expression::from(Value::text("null")))?,
            JsonType::ColumnRef(column) => {
                self.write("JSON_TYPE")?;
                self.surround_with("(", ")", |s| s.visit_column(*column))?;
            }
        }

        self.write(")")
    }

    fn visit_text_search(&mut self, _text_search: crate::prelude::TextSearch<'a>) -> visitor::Result {
        unimplemented!("Full-text search is not yet supported on SQLite")
    }

    fn visit_matches(&mut self, _left: Expression<'a>, _right: Expression<'a>, _not: bool) -> visitor::Result {
        unimplemented!("Full-text search is not yet supported on SQLite")
    }

    fn visit_text_search_relevance(&mut self, _text_search_relevance: TextSearchRelevance<'a>) -> visitor::Result {
        unimplemented!("Full-text search is not yet supported on SQLite")
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_extract_last_array_item(&mut self, extract: JsonExtractLastArrayElem<'a>) -> visitor::Result {
        self.visit_expression(*extract.expr)?;
        self.write("->")?;
        self.visit_parameterized(Value::text("$[#-1]"))
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_extract_first_array_item(&mut self, extract: JsonExtractFirstArrayElem<'a>) -> visitor::Result {
        self.visit_expression(*extract.expr)?;
        self.write("->")?;
        self.visit_parameterized(Value::text("$[0]"))
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_unquote(&mut self, json_unquote: JsonUnquote<'a>) -> visitor::Result {
        self.write("JSONB_EXTRACT")?;
        self.surround_with("(", ")", |s| {
            s.visit_expression(*json_unquote.expr)?;
            s.write(", ")?;
            s.visit_parameterized(Value::text("$"))
        })
    }

    #[cfg(feature = "sqlite")]
    fn visit_json_array_agg(&mut self, array_agg: JsonArrayAgg<'a>) -> visitor::Result {
        self.write("JSONB_GROUP_ARRAY")?;
        self.surround_with("(", ")", |s| s.visit_expression(*array_agg.expr))?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    fn visit_json_build_object(&mut self, build_obj: JsonBuildObject<'a>) -> visitor::Result {
        let len = build_obj.exprs.len();

        self.write("JSONB_OBJECT")?;
        self.surround_with("(", ")", |s| {
            for (i, (name, expr)) in build_obj.exprs.into_iter().enumerate() {
                s.visit_raw_value(Value::text(name))?;
                s.write(", ")?;
                s.visit_expression(expr)?;

                if i < (len - 1) {
                    s.write(", ")?;
                }
            }

            Ok(())
        })?;

        Ok(())
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

        Ok(())
    }

    fn visit_concat(&mut self, concat: Concat<'a>) -> visitor::Result {
        let len = concat.exprs.len();

        self.surround_with("(", ")", |s| {
            for (i, expr) in concat.exprs.into_iter().enumerate() {
                s.visit_expression(expr)?;

                if i < (len - 1) {
                    s.write(" || ")?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn visit_delete(&mut self, delete: Delete<'a>) -> visitor::Result {
        self.write("DELETE FROM ")?;
        self.visit_table(delete.table, true)?;

        if let Some(conditions) = delete.conditions {
            self.write(" WHERE ")?;
            self.visit_conditions(conditions)?;
        }

        self.returning(delete.returning)?;

        if let Some(comment) = delete.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }

    fn visit_update(&mut self, update: Update<'a>) -> visitor::Result {
        self.write("UPDATE ")?;
        self.visit_table(update.table, true)?;

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
        }

        if let Some(conditions) = update.conditions {
            self.write(" WHERE ")?;
            self.visit_conditions(conditions)?;
        }

        self.returning(update.returning)?;

        if let Some(comment) = update.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::visitor::*;

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
        let expected = expected_values("SELECT ?", vec![1]);

        let query = Select::default().value(1);
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_value() {
        let expected = expected_values("SELECT ? AS `test`", vec![1]);

        let query = Select::default().value(val!(1).alias("test"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_null() {
        let expected_sql = "SELECT ? AS `test`";
        let query = Select::default().value(val!(Value::null_text()).alias("test"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::null_text()], params);
    }

    #[test]
    fn test_select_star_from() {
        let expected_sql = "SELECT `musti`.* FROM `musti`";
        let query = Select::from_table("musti");
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_from_values() {
        let expected_sql = "SELECT `vals`.* FROM (VALUES (?,?),(?,?)) AS `vals`";
        let values = Table::from(values!((1, 2), (3, 4))).alias("vals");
        let query = Select::from_table(values);
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![Value::int32(1), Value::int32(2), Value::int32(3), Value::int32(4),],
            params
        );
    }

    #[test]
    fn test_in_values() {
        let expected_sql = "SELECT `test`.* FROM `test` WHERE (`id1`,`id2`) IN (VALUES (?,?),(?,?))";
        let query = Select::from_table("test")
            .so_that(Row::from((col!("id1"), col!("id2"))).in_selection(values!((1, 2), (3, 4))));

        let (sql, params) = Sqlite::build(query).unwrap();

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
        let (sql, params) = Sqlite::build(query).unwrap();
        let expected_sql = "SELECT `test`.* FROM `test` WHERE `id1` IN (?,?)";

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::int32(1), Value::int32(2),], params)
    }

    #[test]
    fn test_select_order_by() {
        let expected_sql = "SELECT `musti`.* FROM `musti` ORDER BY `foo`, `baz` ASC, `bar` DESC";
        let query = Select::from_table("musti")
            .order_by("foo")
            .order_by("baz".ascend())
            .order_by("bar".descend());
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_fields_from() {
        let expected_sql = "SELECT `paw`, `nose` FROM `cat`.`musti`";
        let query = Select::from_table(("cat", "musti")).column("paw").column("nose");
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_where_equals() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` = ?", vec!["meow"]);

        let query = Select::from_table("naukio").so_that("word".equals("meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_like() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` LIKE ?", vec!["%meow%"]);

        let query = Select::from_table("naukio").so_that("word".like("%meow%"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_like() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `word` NOT LIKE ?",
            vec!["%meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".not_like("%meow%"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_begins_with() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` LIKE ?", vec!["%meow"]);

        let query = Select::from_table("naukio").so_that("word".like("%meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_begins_with() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` NOT LIKE ?", vec!["%meow"]);

        let query = Select::from_table("naukio").so_that("word".not_like("%meow"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_ends_into() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` LIKE ?", vec!["meow%"]);

        let query = Select::from_table("naukio").so_that("word".like("meow%"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_ends_into() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` NOT LIKE ?", vec!["meow%"]);

        let query = Select::from_table("naukio").so_that("word".not_like("meow%"));
        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_and() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE (`word` = ? AND `age` < ? AND `paw` = ?)";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = "word".equals("meow").and("age".less_than(10)).and("paw".equals("warm"));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_and_different_execution_order() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE (`word` = ? AND (`age` < ? AND `paw` = ?))";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = "word".equals("meow").and("age".less_than(10).and("paw".equals("warm")));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_or() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE ((`word` = ? OR `age` < ?) AND `paw` = ?)";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = "word".equals("meow").or("age".less_than(10)).and("paw".equals("warm"));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_negation() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE (NOT ((`word` = ? OR `age` < ?) AND `paw` = ?))";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = "word"
            .equals("meow")
            .or("age".less_than(10))
            .and("paw".equals("warm"))
            .not();

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_with_raw_condition_tree() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE (NOT ((`word` = ? OR `age` < ?) AND `paw` = ?))";

        let expected_params = vec![Value::text("meow"), Value::int32(10), Value::text("warm")];

        let conditions = ConditionTree::not("word".equals("meow").or("age".less_than(10)).and("paw".equals("warm")));
        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_simple_inner_join() {
        let expected_sql = "SELECT `users`.* FROM `users` INNER JOIN `posts` ON `users`.`id` = `posts`.`user_id`";

        let query = Select::from_table("users")
            .inner_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_inner_join() {
        let expected_sql = "SELECT `users`.* FROM `users` INNER JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?)";

        let query = Select::from_table("users").inner_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[test]
    fn test_simple_left_join() {
        let expected_sql = "SELECT `users`.* FROM `users` LEFT JOIN `posts` ON `users`.`id` = `posts`.`user_id`";

        let query = Select::from_table("users")
            .left_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_left_join() {
        let expected_sql = "SELECT `users`.* FROM `users` LEFT JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?)";

        let query = Select::from_table("users").left_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[test]
    fn test_column_aliasing() {
        let expected_sql = "SELECT `bar` AS `foo` FROM `meow`";
        let query = Select::from_table("meow").column(Column::new("bar").alias("foo"));
        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT `bar` FROM `test`";
        let query = Select::from_table("test").column(Column::new("bar")).distinct();
        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_distinct_with_subquery() {
        let expected_sql = "SELECT DISTINCT (SELECT ? FROM `test2`), `bar` FROM `test`";
        let query = Select::from_table("test")
            .value(Select::from_table("test2").value(val!(1)))
            .column(Column::new("bar"))
            .distinct();

        let (sql, _) = Sqlite::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_from() {
        let expected_sql = "SELECT `foo`.*, `bar`.`a` FROM `foo`, (SELECT `a` FROM `baz`) AS `bar`";
        let query = Select::default()
            .and_from("foo")
            .and_from(Table::from(Select::from_table("baz").column("a")).alias("bar"))
            .value(Table::from("foo").asterisk())
            .column(("bar", "a"));

        let (sql, _) = Sqlite::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_comment_insert() {
        let expected_sql = "INSERT INTO `users` DEFAULT VALUES /* trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2' */";
        let query = Insert::single_into("users");
        let insert =
            Insert::from(query).comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");

        let (sql, _) = Sqlite::build(insert).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[cfg(feature = "sqlite")]
    fn sqlite_harness() -> ::rusqlite::Connection {
        let conn = ::rusqlite::Connection::open_in_memory().unwrap();

        conn.execute("CREATE TABLE users (id, name TEXT, age REAL, nice INTEGER)", [])
            .unwrap();

        let insert = Insert::single_into("users")
            .value("id", 1)
            .value("name", "Alice")
            .value("age", 42.69)
            .value("nice", true);

        let (sql, params) = Sqlite::build(insert).unwrap();

        conn.execute(&sql, rusqlite::params_from_iter(params.iter())).unwrap();
        conn
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn bind_test_1() {
        let conn = sqlite_harness();

        let conditions = "name".equals("Alice").and("age".less_than(100.0)).and("nice".equals(1));
        let query = Select::from_table("users").so_that(conditions);
        let (sql_str, params) = Sqlite::build(query).unwrap();

        #[derive(Debug)]
        struct Person {
            name: String,
            age: f64,
            nice: i32,
        }

        let mut stmt = conn.prepare(&sql_str).unwrap();
        let mut person_iter = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok(Person {
                    name: row.get(1).unwrap(),
                    age: row.get(2).unwrap(),
                    nice: row.get(3).unwrap(),
                })
            })
            .unwrap();

        let person: Person = person_iter.next().unwrap().unwrap();

        assert_eq!("Alice", person.name);
        assert_eq!(42.69, person.age);
        assert_eq!(1, person.nice);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) = Sqlite::build(Select::default().value(Value::null_text().raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = Sqlite::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_real() {
        let (sql, params) = Sqlite::build(Select::default().value(1.3f64.raw())).unwrap();
        assert_eq!("SELECT 1.3", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text() {
        let (sql, params) = Sqlite::build(Select::default().value("foo".raw())).unwrap();
        assert_eq!("SELECT 'foo'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_bytes() {
        let (sql, params) = Sqlite::build(Select::default().value(Value::bytes(vec![1, 2, 3]).raw())).unwrap();
        assert_eq!("SELECT x'010203'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = Sqlite::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT true", sql);
        assert!(params.is_empty());

        let (sql, params) = Sqlite::build(Select::default().value(false.raw())).unwrap();
        assert_eq!("SELECT false", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_char() {
        let (sql, params) = Sqlite::build(Select::default().value(Value::character('a').raw())).unwrap();
        assert_eq!("SELECT 'a'", sql);
        assert!(params.is_empty());
    }

    #[test]

    fn test_raw_json() {
        let (sql, params) = Sqlite::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw())).unwrap();
        assert_eq!("SELECT '{\"foo\":\"bar\"}'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let (sql, params) = Sqlite::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", uuid.hyphenated()), sql);

        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_datetime() {
        let dt = chrono::Utc::now();
        let (sql, params) = Sqlite::build(Select::default().value(dt.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", dt.to_rfc3339(),), sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_default_insert() {
        let insert = Insert::single_into("foo")
            .value("foo", "bar")
            .value("baz", default_value());

        let (sql, _) = Sqlite::build(insert).unwrap();

        assert_eq!("INSERT INTO `foo` (`foo`, `baz`) VALUES (?,DEFAULT)", sql);
    }

    #[test]
    fn join_is_inserted_positionally() {
        let joined_table = Table::from("User").left_join(
            "Post"
                .alias("p")
                .on(("p", "userId").equals(Column::from(("User", "id")))),
        );
        let q = Select::from_table(joined_table).and_from("Toto");
        let (sql, _) = Sqlite::build(q).unwrap();

        assert_eq!(
            "SELECT `User`.*, `Toto`.* FROM `User` LEFT JOIN `Post` AS `p` ON `p`.`userId` = `User`.`id`, `Toto`",
            sql
        );
    }

    #[test]
    fn test_returning() {
        let insert = Insert::single_into("test").value("user id", 1).value("txt", "hello");
        let insert: Insert = Insert::from(insert).returning(["user id"]);

        let (sql, _) = Sqlite::build(insert).unwrap();

        assert_eq!(
            "INSERT INTO `test` (`user id`, `txt`) VALUES (?,?) RETURNING `user id` AS `user id`",
            sql
        );
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn test_insert_on_conflict_update() {
        let expected = expected_values(
            "INSERT INTO \"users\" (\"foo\") VALUES ($1) ON CONFLICT (\"foo\") DO UPDATE SET \"foo\" = $2 WHERE \"users\".\"foo\" = $3 RETURNING \"foo\"",
            vec![10, 3, 1],
        );

        let update = Update::table("users").set("foo", 3).so_that(("users", "foo").equals(1));

        let query: Insert = Insert::single_into("users").value("foo", 10).into();

        let query = query.on_conflict(OnConflict::Update(update, Vec::from(["foo".into()])));

        let (sql, params) = Postgres::build(query.returning(vec!["foo"])).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }
}
