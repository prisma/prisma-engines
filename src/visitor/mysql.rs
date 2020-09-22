use crate::{
    ast::*,
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use std::fmt::{self, Write};

/// A visitor to generate queries for the MySQL database.
///
/// The returned parameter values can be used directly with the mysql crate.
pub struct Mysql<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
}

impl<'a> Mysql<'a> {
    fn visit_regular_equality_comparison(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        self.visit_expression(left)?;
        self.write(" = ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    fn visit_regular_difference_comparison(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        self.visit_expression(left)?;
        self.write(" <> ")?;
        self.visit_expression(right)?;

        Ok(())
    }
}

impl<'a> Visitor<'a> for Mysql<'a> {
    const C_BACKTICK_OPEN: &'static str = "`";
    const C_BACKTICK_CLOSE: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> crate::Result<(String, Vec<Value<'a>>)>
    where
        Q: Into<Query<'a>>,
    {
        let mut mysql = Mysql {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
        };

        Mysql::visit_query(&mut mysql, query.into())?;

        Ok((mysql.query, mysql.parameters))
    }

    fn write<D: fmt::Display>(&mut self, s: D) -> visitor::Result {
        write!(&mut self.query, "{}", s)?;
        Ok(())
    }

    fn visit_raw_value(&mut self, value: Value<'a>) -> visitor::Result {
        let res = match value {
            Value::Integer(i) => i.map(|i| self.write(i)),
            Value::Real(r) => r.map(|r| self.write(r)),
            Value::Text(t) => t.map(|t| self.write(format!("'{}'", t))),
            Value::Enum(e) => e.map(|e| self.write(e)),
            Value::Bytes(b) => b.map(|b| self.write(format!("x'{}'", hex::encode(b)))),
            Value::Boolean(b) => b.map(|b| self.write(b)),
            Value::Char(c) => c.map(|c| self.write(format!("'{}'", c))),
            #[cfg(feature = "json-1")]
            Value::Json(j) => match j {
                Some(ref j) => {
                    let s = serde_json::to_string(&j)?;
                    Some(self.write(format!("CONVERT('{}', JSON)", s)))
                }
                None => None,
            },
            #[cfg(all(feature = "array", feature = "postgresql"))]
            Value::Array(_) => {
                let msg = "Arrays are not supported in MySQL.";
                let kind = ErrorKind::conversion(msg);

                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())?
            }
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(uuid) => uuid.map(|uuid| self.write(format!("'{}'", uuid.to_hyphenated().to_string()))),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(dt) => dt.map(|dt| self.write(format!("'{}'", dt.to_rfc3339(),))),
            #[cfg(feature = "chrono-0_4")]
            Value::Date(date) => date.map(|date| self.write(format!("'{}'", date))),
            #[cfg(feature = "chrono-0_4")]
            Value::Time(time) => time.map(|time| self.write(format!("'{}'", time))),
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitor::Result {
        match insert.on_conflict {
            Some(OnConflict::DoNothing) => self.write("INSERT IGNORE ")?,
            None => self.write("INSERT ")?,
        };

        if let Some(table) = insert.table {
            self.write("INTO ")?;
            self.visit_table(table, true)?;
        }

        match insert.values {
            Expression {
                kind: ExpressionKind::Row(row),
                ..
            } => {
                if row.values.is_empty() {
                    self.write(" () VALUES ()")?;
                } else {
                    let columns = insert.columns.len();

                    self.write(" (")?;
                    for (i, c) in insert.columns.into_iter().enumerate() {
                        self.visit_column(c)?;

                        if i < (columns - 1) {
                            self.write(",")?;
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
                    self.visit_column(c)?;

                    if i < (columns - 1) {
                        self.write(",")?;
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
            expr => self.surround_with("(", ")", |ref mut s| s.visit_expression(expr))?,
        }

        Ok(())
    }

    fn parameter_substitution(&mut self) -> visitor::Result {
        self.write("?")
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value);
    }

    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> visitor::Result {
        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (None, Some(Value::Integer(Some(offset)))) if offset < 1 => Ok(()),
            (None, Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(Value::from(9_223_372_036_854_775_807i64))?;

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
        self.write(" GROUP_CONCAT")?;
        self.surround_with("(", ")", |ref mut s| s.visit_expression(value))
    }

    fn visit_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        #[cfg(feature = "json-1")]
        {
            if right.is_json_value() || left.is_json_value() {
                self.write("JSON_CONTAINS")?;
                self.surround_with("(", ")", |s| {
                    s.visit_expression(left.clone())?;
                    s.write(", ")?;
                    s.visit_expression(right.clone())
                })?;

                self.write(" AND ")?;

                self.write("JSON_CONTAINS")?;
                self.surround_with("(", ")", |s| {
                    s.visit_expression(right)?;
                    s.write(", ")?;
                    s.visit_expression(left)
                })
            } else {
                self.visit_regular_equality_comparison(left, right)
            }
        }

        #[cfg(not(feature = "json-1"))]
        {
            self.visit_regular_equality_comparison(left, right)
        }
    }

    fn visit_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        #[cfg(feature = "json-1")]
        {
            if right.is_json_value() || left.is_json_value() {
                self.write("NOT JSON_CONTAINS")?;
                self.surround_with("(", ")", |s| {
                    s.visit_expression(left.clone())?;
                    s.write(", ")?;
                    s.visit_expression(right.clone())
                })?;

                self.write(" OR ")?;

                self.write("NOT JSON_CONTAINS")?;
                self.surround_with("(", ")", |s| {
                    s.visit_expression(right)?;
                    s.write(", ")?;
                    s.visit_expression(left)
                })
            } else {
                self.visit_regular_difference_comparison(left, right)
            }
        }

        #[cfg(not(feature = "json-1"))]
        {
            self.visit_regular_difference_comparison(left, right)
        }
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

    fn default_params<'a>(mut additional: Vec<Value<'a>>) -> Vec<Value<'a>> {
        let mut result = Vec::new();

        for param in additional.drain(0..) {
            result.push(param)
        }

        result
    }

    #[test]
    fn test_single_row_insert_default_values() {
        let query = Insert::single_into("users");
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!("INSERT INTO `users` () VALUES ()", sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_single_row_insert() {
        let expected = expected_values("INSERT INTO `users` (`foo`) VALUES (?)", vec![10]);
        let query = Insert::single_into("users").value("foo", 10);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_multi_row_insert() {
        let expected = expected_values("INSERT INTO `users` (`foo`) VALUES (?), (?)", vec![10, 11]);
        let query = Insert::multi_into("users", vec!["foo"])
            .values(vec![10])
            .values(vec![11]);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_both_are_set() {
        let expected = expected_values("SELECT `users`.* FROM `users` LIMIT ? OFFSET ?", vec![10, 2]);
        let query = Select::from_table("users").limit(10).offset(2);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_offset_is_set() {
        let expected = expected_values(
            "SELECT `users`.* FROM `users` LIMIT ? OFFSET ?",
            vec![9_223_372_036_854_775_807i64, 10],
        );

        let query = Select::from_table("users").offset(10);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_limit_is_set() {
        let expected = expected_values("SELECT `users`.* FROM `users` LIMIT ?", vec![10]);
        let query = Select::from_table("users").limit(10);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_in_values_2_tuple() {
        use crate::{col, values};

        let expected_sql = "SELECT `test`.* FROM `test` WHERE (`id1`,`id2`) IN ((?,?),(?,?))";
        let query = Select::from_table("test")
            .so_that(Row::from((col!("id1"), col!("id2"))).in_selection(values!((1, 2), (3, 4))));

        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(3),
                Value::integer(4),
            ],
            params
        );
    }

    #[cfg(feature = "json-1")]
    #[test]
    fn equality_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT `users`.* FROM `users` WHERE JSON_CONTAINS(`jsonField`, ?) AND JSON_CONTAINS(?, `jsonField`)"#,
            vec![serde_json::json!({"a": "b"}), serde_json::json!({"a": "b"})],
        );

        let query = Select::from_table("users").so_that(Column::from("jsonField").equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[cfg(feature = "json-1")]
    #[test]
    fn difference_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT `users`.* FROM `users` WHERE NOT JSON_CONTAINS(`jsonField`, ?) OR NOT JSON_CONTAINS(?, `jsonField`)"#,
            vec![serde_json::json!({"a": "b"}), serde_json::json!({"a": "b"})],
        );

        let query =
            Select::from_table("users").so_that(Column::from("jsonField").not_equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) = Mysql::build(Select::default().value(Value::Text(None).raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = Mysql::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_real() {
        let (sql, params) = Mysql::build(Select::default().value(1.3f64.raw())).unwrap();
        assert_eq!("SELECT 1.3", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text() {
        let (sql, params) = Mysql::build(Select::default().value("foo".raw())).unwrap();
        assert_eq!("SELECT 'foo'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_bytes() {
        let (sql, params) = Mysql::build(Select::default().value(Value::bytes(vec![1, 2, 3]).raw())).unwrap();
        assert_eq!("SELECT x'010203'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = Mysql::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT true", sql);
        assert!(params.is_empty());

        let (sql, params) = Mysql::build(Select::default().value(false.raw())).unwrap();
        assert_eq!("SELECT false", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_char() {
        let (sql, params) = Mysql::build(Select::default().value(Value::character('a').raw())).unwrap();
        assert_eq!("SELECT 'a'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT `bar` FROM `test`";
        let query = Select::from_table("test").column(Column::new("bar")).distinct();
        let (sql, _) = Mysql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_distinct_with_subquery() {
        let expected_sql = "SELECT DISTINCT (SELECT ? FROM `test2`), `bar` FROM `test`";
        let query = Select::from_table("test")
            .value(Select::from_table("test2").value(val!(1)))
            .column(Column::new("bar"))
            .distinct();

        let (sql, _) = Mysql::build(query).unwrap();

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

        let (sql, _) = Mysql::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    #[cfg(feature = "json-1")]
    fn test_raw_json() {
        let (sql, params) = Mysql::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw())).unwrap();
        assert_eq!("SELECT CONVERT('{\"foo\":\"bar\"}', JSON)", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "uuid-0_8")]
    fn test_raw_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let (sql, params) = Mysql::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", uuid.to_hyphenated().to_string()), sql);

        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "chrono-0_4")]
    fn test_raw_datetime() {
        let dt = chrono::Utc::now();
        let (sql, params) = Mysql::build(Select::default().value(dt.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", dt.to_rfc3339(),), sql);
        assert!(params.is_empty());
    }
}
