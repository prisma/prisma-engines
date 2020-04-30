use crate::{ast::*, visitor::Visitor};
use std::fmt::{self, Write};

/// A visitor to generate queries for the MySQL database.
///
/// The returned parameter values can be used directly with the mysql crate.
pub struct Mysql<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
}

impl<'a> Mysql<'a> {
    fn visit_regular_equality_comparison(&mut self, left: Expression<'a>, right: Expression<'a>) -> fmt::Result {
        self.visit_expression(left)?;
        self.write(" = ")?;
        self.visit_expression(right)
    }

    fn visit_regular_difference_comparison(&mut self, left: Expression<'a>, right: Expression<'a>) -> fmt::Result {
        self.visit_expression(left)?;
        self.write(" <> ")?;
        self.visit_expression(right)
    }
}

impl<'a> Visitor<'a> for Mysql<'a> {
    const C_BACKTICK: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> (String, Vec<Value<'a>>)
    where
        Q: Into<Query<'a>>,
    {
        let mut mysql = Mysql {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
        };

        Mysql::visit_query(&mut mysql, query.into());

        (mysql.query, mysql.parameters)
    }

    fn write<D: fmt::Display>(&mut self, s: D) -> fmt::Result {
        write!(&mut self.query, "{}", s)
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> fmt::Result {
        match insert.on_conflict {
            Some(OnConflict::DoNothing) => self.write("INSERT IGNORE INTO ")?,
            None => self.write("INSERT INTO ")?,
        };

        self.visit_table(insert.table, true)?;

        if insert.values.is_empty() {
            self.write(" () VALUES ()")
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
            let values = insert.values.len();

            for (i, row) in insert.values.into_iter().enumerate() {
                self.visit_row(row)?;

                if i < (values - 1) {
                    self.write(", ")?;
                }
            }

            Ok(())
        }
    }

    fn parameter_substitution(&mut self) -> fmt::Result {
        self.write("?")
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value);
    }

    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> fmt::Result {
        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (None, Some(Value::Integer(offset))) if offset < 1 => Ok(()),
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

    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> fmt::Result {
        self.write(" GROUP_CONCAT")?;
        self.surround_with("(", ")", |ref mut s| s.visit_expression(value))
    }

    fn visit_condition_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> fmt::Result {
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

    fn visit_condition_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> fmt::Result {
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
        let (sql, params) = Mysql::build(query);

        assert_eq!("INSERT INTO `users` () VALUES ()", sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_single_row_insert() {
        let expected = expected_values("INSERT INTO `users` (`foo`) VALUES (?)", vec![10]);
        let query = Insert::single_into("users").value("foo", 10);
        let (sql, params) = Mysql::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_multi_row_insert() {
        let expected = expected_values("INSERT INTO `users` (`foo`) VALUES (?), (?)", vec![10, 11]);
        let query = Insert::multi_into("users", vec!["foo"])
            .values(vec![10])
            .values(vec![11]);
        let (sql, params) = Mysql::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_both_are_set() {
        let expected = expected_values("SELECT `users`.* FROM `users` LIMIT ? OFFSET ?", vec![10, 2]);
        let query = Select::from_table("users").limit(10).offset(2);
        let (sql, params) = Mysql::build(query);

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
        let (sql, params) = Mysql::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_limit_is_set() {
        let expected = expected_values("SELECT `users`.* FROM `users` LIMIT ?", vec![10]);
        let query = Select::from_table("users").limit(10);
        let (sql, params) = Mysql::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_in_values_2_tuple() {
        use crate::{col, values};

        let expected_sql = "SELECT `test`.* FROM `test` WHERE (`id1`,`id2`) IN ((?,?),(?,?))";
        let query = Select::from_table("test")
            .so_that(Row::from((col!("id1"), col!("id2"))).in_selection(values!((1, 2), (3, 4))));

        let (sql, params) = Mysql::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
                Value::Integer(4),
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
        let (sql, params) = Mysql::build(query);

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
        let (sql, params) = Mysql::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }
}
