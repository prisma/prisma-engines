use crate::{
    ast::*,
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};

use crate::visitor::query_writer::QueryWriter;
use query_template::{PlaceholderFormat, QueryTemplate};
use std::{borrow::Cow, fmt};

/// A visitor to generate queries for SurrealDB using SurrealQL syntax.
///
/// SurrealQL is similar to SQL but has key differences:
/// - Uses `$1`, `$2` style placeholders (like PostgreSQL)
/// - Uses backtick identifiers (like MySQL/SQLite)
/// - DELETE syntax omits FROM: `DELETE table WHERE ...`
/// - INSERT uses `INSERT INTO table { field: value }` or standard SQL syntax
/// - RETURNING is `RETURN AFTER` / `RETURN BEFORE` / `RETURN NONE`
pub struct SurrealDb<'a> {
    query_template: QueryTemplate<Value<'a>>,
}

impl<'a> SurrealDb<'a> {
    fn returning(&mut self, returning: Option<Vec<Column<'a>>>) -> visitor::Result {
        if let Some(returning) = returning
            && !returning.is_empty()
        {
            let values_len = returning.len();
            self.write(" RETURN ")?;

            for (i, column) in returning.into_iter().enumerate() {
                self.surround_with_backticks(&column.name)?;
                if i < (values_len - 1) {
                    self.write(", ")?;
                }
            }
        }
        Ok(())
    }
}

impl<'a> SurrealDb<'a> {
    fn visit_order_by(&mut self, direction: &str, value: Expression<'a>) -> visitor::Result {
        self.visit_expression(value)?;
        self.write(format!(" {direction}"))?;
        Ok(())
    }

    fn visit_order_by_nulls_first(&mut self, direction: &str, value: Expression<'a>) -> visitor::Result {
        self.surround_with("CASE WHEN ", " END", |s| {
            s.visit_expression(value.clone())?;
            s.write(" IS NULL THEN 0 ELSE 1")
        })?;
        self.write(", ")?;
        self.visit_order_by(direction, value)?;
        Ok(())
    }

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

impl<'a> Visitor<'a> for SurrealDb<'a> {
    const C_BACKTICK_OPEN: &'static str = "`";
    const C_BACKTICK_CLOSE: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    fn build_template<Q>(query: Q) -> crate::Result<QueryTemplate<Value<'a>>>
    where
        Q: Into<Query<'a>>,
    {
        let mut this = SurrealDb {
            // SurrealDB requires parameter names to start with a letter.
            // We use "$p" prefix with numbering: $p1, $p2, ...
            query_template: QueryTemplate::new(PlaceholderFormat {
                prefix: "$p",
                has_numbering: true,
            }),
        };

        SurrealDb::visit_query(&mut this, query.into())?;

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
            ValueType::Text(t) => t.as_ref().map(|t| self.write(format!("'{}'", escape_squote(t)))),
            ValueType::Enum(e, _) => e.as_ref().map(|e| self.write(format!("'{}'", escape_squote(e)))),
            ValueType::Bytes(b) => b.as_ref().map(|b| self.write(format!("x'{}'", hex::encode(b)))),
            ValueType::Boolean(b) => b.map(|b| self.write(b)),
            ValueType::Char(c) => c.map(|c| self.write(format!("'{}'", escape_squote(&c.to_string())))),
            ValueType::Float(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f32::INFINITY => self.write("'Infinity'"),
                f if f == f32::NEG_INFINITY => self.write("'-Infinity'"),
                v => self.write(format!("{v:?}")),
            }),
            ValueType::Double(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f64::INFINITY => self.write("'Infinity'"),
                f if f == f64::NEG_INFINITY => self.write("'-Infinity'"),
                v => self.write(format!("{v:?}")),
            }),
            ValueType::Array(_) | ValueType::EnumArray(_, _) => {
                let msg = "Arrays are not supported in SurrealDB visitor.";
                let kind = ErrorKind::conversion(msg);
                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);
                return Err(builder.build());
            }
            ValueType::Json(j) => match j {
                Some(j) => {
                    let s = serde_json::to_string(j)?;
                    Some(self.write(format!("'{}'", escape_squote(&s))))
                }
                None => None,
            },
            ValueType::Numeric(r) => r.as_ref().map(|r| self.write(r)),
            ValueType::Uuid(uuid) => uuid.map(|uuid| self.write(format!("'{}'", uuid.hyphenated()))),
            ValueType::DateTime(dt) => dt.map(|dt| self.write(format!("'{}'", dt.to_rfc3339()))),
            ValueType::Date(date) => date.map(|date| self.write(format!("'{}'", escape_squote(&date.to_string())))),
            ValueType::Time(time) => time.map(|time| self.write(format!("'{}'", escape_squote(&time.to_string())))),
            ValueType::Xml(cow) => cow.as_ref().map(|cow| self.write(format!("'{}'", escape_squote(cow)))),
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
            Some(OnConflict::DoNothing) => self.write("INSERT IGNORE")?,
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

        if let Some(OnConflict::Update(update, _constraints)) = insert.on_conflict {
            // SurrealDB uses ON DUPLICATE KEY UPDATE (not PostgreSQL ON CONFLICT DO)
            self.write(" ON DUPLICATE KEY UPDATE ")?;
            self.visit_update_set(update)?;
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
                self.write(" START ")?;
                self.visit_parameterized(offset)
            }
            (None, Some(offset)) => {
                self.write(" START ")?;
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
        self.write("string::join")?;
        self.surround_with("(', ', ", ")", |ref mut s| s.visit_expression(value))
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

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite", feature = "surrealdb"))]
    fn visit_json_extract(&mut self, json_extract: JsonExtract<'a>) -> visitor::Result {
        // SurrealDB uses dot-notation for field access: column.path
        self.visit_expression(*json_extract.column)?;

        match json_extract.path {
            JsonPath::String(path) => {
                self.write(".")?;
                self.write(path)?;
            }
            JsonPath::Array(parts) => {
                for part in parts {
                    self.write(".")?;
                    self.write(part)?;
                }
            }
        }

        Ok(())
    }

    fn visit_json_array_contains(
        &mut self,
        left: Expression<'a>,
        right: Expression<'a>,
        not: bool,
    ) -> visitor::Result {
        // SurrealDB: <array> CONTAINS <value>
        self.visit_expression(left)?;
        if not {
            self.write(" CONTAINSNOT ")?;
        } else {
            self.write(" CONTAINS ")?;
        }
        self.visit_expression(right)
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite", feature = "surrealdb"))]
    fn visit_json_type_equals(&mut self, left: Expression<'a>, json_type: JsonType<'a>, not: bool) -> visitor::Result {
        // SurrealDB: type::is_<type>(value) returns bool
        let type_fn = match json_type {
            JsonType::Array => "type::is_array",
            JsonType::Boolean => "type::is_bool",
            JsonType::Number => "type::is_number",
            JsonType::Object => "type::is_object",
            JsonType::String => "type::is_string",
            JsonType::Null => "type::is_null",
            JsonType::ColumnRef(_) => {
                let msg = "Dynamic JSON type comparison via column reference is not supported in SurrealDB";
                return Err(Error::builder(ErrorKind::conversion(msg)).build());
            }
        };

        if not {
            self.write("NOT ")?;
        }
        self.write(type_fn)?;
        self.surround_with("(", ")", |s| s.visit_expression(left))
    }

    fn visit_text_search(&mut self, _text_search: crate::prelude::TextSearch<'a>) -> visitor::Result {
        Err(Error::builder(ErrorKind::conversion(
            "Full-text search is not yet supported for SurrealDB",
        )).build())
    }

    fn visit_matches(&mut self, _left: Expression<'a>, _right: Expression<'a>, _not: bool) -> visitor::Result {
        Err(Error::builder(ErrorKind::conversion(
            "Full-text search MATCHES is not yet supported for SurrealDB",
        )).build())
    }

    fn visit_text_search_relevance(&mut self, _text_search_relevance: TextSearchRelevance<'a>) -> visitor::Result {
        Err(Error::builder(ErrorKind::conversion(
            "Full-text search relevance is not yet supported for SurrealDB",
        )).build())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite", feature = "surrealdb"))]
    fn visit_json_extract_last_array_item(&mut self, extract: JsonExtractLastArrayElem<'a>) -> visitor::Result {
        // SurrealDB: array::last(expr)
        self.write("array::last")?;
        self.surround_with("(", ")", |s| s.visit_expression(*extract.expr))
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite", feature = "surrealdb"))]
    fn visit_json_extract_first_array_item(&mut self, extract: JsonExtractFirstArrayElem<'a>) -> visitor::Result {
        // SurrealDB: array::first(expr)
        self.write("array::first")?;
        self.surround_with("(", ")", |s| s.visit_expression(*extract.expr))
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite", feature = "surrealdb"))]
    fn visit_json_unquote(&mut self, json_unquote: JsonUnquote<'a>) -> visitor::Result {
        // SurrealDB: <string> expr or type::string(expr)
        self.write("type::string")?;
        self.surround_with("(", ")", |s| s.visit_expression(*json_unquote.expr))
    }

    #[cfg(feature = "surrealdb")]
    fn visit_json_array_agg(&mut self, array_agg: JsonArrayAgg<'a>) -> visitor::Result {
        self.write("array::group")?;
        self.surround_with("(", ")", |s| s.visit_expression(*array_agg.expr))?;
        Ok(())
    }

    #[cfg(feature = "surrealdb")]
    fn visit_json_build_object(&mut self, build_obj: JsonBuildObject<'a>) -> visitor::Result {
        let len = build_obj.exprs.len();

        self.write("{ ")?;
        for (i, (name, expr)) in build_obj.exprs.into_iter().enumerate() {
            self.write(format!("{name}: "))?;
            self.visit_expression(expr)?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }
        self.write(" }")?;

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

        self.write("string::concat")?;
        self.surround_with("(", ")", |s| {
            for (i, expr) in concat.exprs.into_iter().enumerate() {
                s.visit_expression(expr)?;
                if i < (len - 1) {
                    s.write(", ")?;
                }
            }
            Ok(())
        })?;

        Ok(())
    }

    fn visit_delete(&mut self, delete: Delete<'a>) -> visitor::Result {
        // SurrealQL: DELETE table WHERE ... (no FROM keyword)
        self.write("DELETE ")?;
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

/// Escape single quotes in string literals for SurrealQL.
fn escape_squote(s: &str) -> Cow<'_, str> {
    let needs_escape = s.contains('\\') || s.contains('\'');
    if needs_escape {
        // Escape backslashes first, then single quotes, to prevent injection
        // via sequences like \' which would otherwise become \\' (breaking out).
        Cow::Owned(s.replace('\\', "\\\\").replace('\'', "\\'"))
    } else {
        Cow::Borrowed(s)
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

    fn default_params(additional: Vec<Value<'_>>) -> Vec<Value<'_>> {
        additional
    }

    #[test]
    fn test_select_1() {
        let expected = expected_values("SELECT $p1", vec![1]);
        let query = Select::default().value(1);
        let (sql, params) = SurrealDb::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_star_from() {
        let expected_sql = "SELECT `musti`.* FROM `musti`";
        let query = Select::from_table("musti");
        let (sql, params) = SurrealDb::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_where_equals() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` = $p1", vec!["meow"]);
        let query = Select::from_table("naukio").so_that("word".equals("meow"));
        let (sql, params) = SurrealDb::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_delete_without_from() {
        let expected_sql = "DELETE `users` WHERE `id` = $p1";
        let query = Delete::from_table("users").so_that("id".equals(1));
        let (sql, params) = SurrealDb::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::int32(1)], params);
    }

    #[test]
    fn test_update() {
        let expected_sql = "UPDATE `users` SET `name` = $p1 WHERE `id` = $p2";
        let query = Update::table("users").set("name", "Alice").so_that("id".equals(1));
        let (sql, params) = SurrealDb::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::text("Alice"), Value::int32(1)], params);
    }

    #[test]
    fn test_insert() {
        let insert = Insert::single_into("users").value("name", "Alice").value("age", 30);
        let (sql, params) = SurrealDb::build(insert).unwrap();

        assert_eq!("INSERT INTO `users` (`name`, `age`) VALUES ($p1,$p2)", sql);
        assert_eq!(vec![Value::text("Alice"), Value::int32(30)], params);
    }

    #[test]
    fn test_limit_and_offset_uses_start() {
        let expected_sql = "SELECT `users`.* FROM `users` LIMIT $p1 START $p2";
        let query = Select::from_table("users").limit(10).offset(20);
        let (sql, params) = SurrealDb::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::int64(10), Value::int64(20)], params);
    }

    #[test]
    fn test_select_order_by() {
        let expected_sql = "SELECT `users`.* FROM `users` ORDER BY `name` ASC, `age` DESC";
        let query = Select::from_table("users")
            .order_by("name".ascend())
            .order_by("age".descend());
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_select_fields() {
        let expected_sql = "SELECT `name`, `email` FROM `users`";
        let query = Select::from_table("users").column("name").column("email");
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_select_and_conditions() {
        let expected_sql = "SELECT `users`.* FROM `users` WHERE (`name` = $p1 AND `age` > $p2)";
        let query = Select::from_table("users")
            .so_that("name".equals("Alice").and("age".greater_than(20)));
        let (sql, params) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::text("Alice"), Value::int32(20)], params);
    }

    #[test]
    fn test_select_or_conditions() {
        let expected_sql = "SELECT `users`.* FROM `users` WHERE (`name` = $p1 OR `name` = $p2)";
        let query = Select::from_table("users")
            .so_that("name".equals("Alice").or("name".equals("Bob")));
        let (sql, params) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::text("Alice"), Value::text("Bob")], params);
    }

    #[test]
    fn test_select_not() {
        let expected_sql = "SELECT `users`.* FROM `users` WHERE (NOT `active` = $p1)";
        let query = Select::from_table("users").so_that("active".equals(false).not());
        let (sql, params) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::boolean(false)], params);
    }

    #[test]
    fn test_inner_join() {
        let expected_sql = "SELECT `users`.* FROM `users` INNER JOIN `posts` ON `users`.`id` = `posts`.`user_id`";
        let query = Select::from_table("users")
            .inner_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_left_join() {
        let expected_sql = "SELECT `users`.* FROM `users` LEFT JOIN `posts` ON `users`.`id` = `posts`.`user_id`";
        let query = Select::from_table("users")
            .left_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_aliased_column() {
        let expected_sql = "SELECT `name` AS `user_name` FROM `users`";
        let query = Select::from_table("users").column(Column::new("name").alias("user_name"));
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT `name` FROM `users`";
        let query = Select::from_table("users").column(Column::new("name")).distinct();
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_in_values() {
        let expected_sql = "SELECT `users`.* FROM `users` WHERE `id` IN ($p1,$p2,$p3)";
        let query = Select::from_table("users").so_that("id".in_selection(vec![1, 2, 3]));
        let (sql, params) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![Value::int32(1), Value::int32(2), Value::int32(3)],
            params
        );
    }

    #[test]
    fn test_is_null() {
        let expected_sql = "SELECT `users`.* FROM `users` WHERE `name` IS NULL";
        let query = Select::from_table("users").so_that("name".is_null());
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_is_not_null() {
        let expected_sql = "SELECT `users`.* FROM `users` WHERE `name` IS NOT NULL";
        let query = Select::from_table("users").so_that("name".is_not_null());
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_like() {
        let expected = expected_values(
            "SELECT `users`.* FROM `users` WHERE `name` LIKE $p1",
            vec!["%alice%"],
        );
        let query = Select::from_table("users").so_that("name".like("%alice%"));
        let (sql, params) = SurrealDb::build(query).unwrap();
        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) = SurrealDb::build(Select::default().value(Value::null_text().raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = SurrealDb::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text_with_quotes() {
        let (sql, params) = SurrealDb::build(Select::default().value("O'Reilly".raw())).unwrap();
        assert_eq!("SELECT 'O\\'Reilly'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text_with_backslash() {
        let (sql, params) = SurrealDb::build(Select::default().value("back\\slash".raw())).unwrap();
        assert_eq!("SELECT 'back\\\\slash'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = SurrealDb::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT true", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_insert_default_values() {
        let insert = Insert::single_into("users");
        let (sql, _) = SurrealDb::build(insert).unwrap();
        assert_eq!("INSERT INTO `users` DEFAULT VALUES", sql);
    }

    #[test]
    fn test_insert_on_duplicate_key_update() {
        let update = Update::table("users").set("name", "Alice Updated").so_that(("users", "id").equals(1));
        let query: Insert = Insert::single_into("users").value("name", "Alice").into();
        let query = query.on_conflict(OnConflict::Update(update, Vec::from(["name".into()])));
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert!(sql.contains("ON DUPLICATE KEY UPDATE"), "Expected ON DUPLICATE KEY UPDATE, got: {sql}");
    }

    #[test]
    fn test_comment() {
        let expected_sql = "INSERT INTO `users` DEFAULT VALUES /* trace_id='abc123' */";
        let query = Insert::single_into("users");
        let insert = Insert::from(query).comment("trace_id='abc123'");
        let (sql, _) = SurrealDb::build(insert).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_returning() {
        let insert = Insert::single_into("test").value("name", "hello");
        let insert: Insert = Insert::from(insert).returning(["name"]);
        let (sql, _) = SurrealDb::build(insert).unwrap();
        assert!(sql.contains("RETURN `name`"), "Expected RETURN clause, got: {sql}");
    }

    #[test]
    fn test_update_without_where() {
        let expected_sql = "UPDATE `users` SET `active` = $p1";
        let query = Update::table("users").set("active", true);
        let (sql, params) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::boolean(true)], params);
    }

    #[test]
    fn test_delete_all() {
        let expected_sql = "DELETE `users`";
        let query = Delete::from_table("users");
        let (sql, _) = SurrealDb::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }
}
