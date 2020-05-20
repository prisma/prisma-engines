use crate::{ast::*, visitor::Visitor};
use std::fmt::{self, Write};

/// A visitor to generate queries for the PostgreSQL database.
///
/// The returned parameter values implement the `ToSql` trait from postgres and
/// can be used directly with the database.
pub struct Postgres<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
}

impl<'a> Visitor<'a> for Postgres<'a> {
    const C_BACKTICK: &'static str = "\"";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> (String, Vec<Value<'a>>)
    where
        Q: Into<Query<'a>>,
    {
        let mut postgres = Postgres {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
        };

        Postgres::visit_query(&mut postgres, query.into());

        (postgres.query, postgres.parameters)
    }

    fn write<D: fmt::Display>(&mut self, s: D) -> fmt::Result {
        write!(&mut self.query, "{}", s)
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value);
    }

    fn parameter_substitution(&mut self) -> fmt::Result {
        self.write("$")?;
        self.write(self.parameters.len())
    }

    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> fmt::Result {
        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (None, Some(offset)) => {
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

    fn visit_insert(&mut self, insert: Insert<'a>) -> fmt::Result {
        self.write("INSERT INTO ")?;
        self.visit_table(insert.table, true)?;

        if insert.values.is_empty() {
            self.write(" DEFAULT VALUES")?;
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
        }

        if let Some(OnConflict::DoNothing) = insert.on_conflict {
            self.write(" ON CONFLICT DO NOTHING")?;
        };

        if let Some(returning) = insert.returning {
            if !returning.is_empty() {
                let values = returning.into_iter().map(|r| r.into()).collect();
                self.write(" RETURNING ")?;
                self.visit_columns(values)?;
            }
        };

        Ok(())
    }

    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> fmt::Result {
        self.write("ARRAY_TO_STRING")?;
        self.write("(")?;
        self.write("ARRAY_AGG")?;
        self.write("(")?;
        self.visit_expression(value)?;
        self.write(")")?;
        self.write("','")?;
        self.write(")")
    }

    #[cfg(feature = "json-1")]
    fn visit_condition_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> fmt::Result {
        let (left_is_json, right_is_json) = (left.is_json_value(), right.is_json_value());

        self.visit_expression(left)?;

        if right_is_json {
            self.write("::jsonb")?;
        }

        self.write(" = ")?;

        if left_is_json {
            self.write("::jsonb")?;
        }

        self.visit_expression(right)
    }

    #[cfg(not(feature = "json-1"))]
    fn visit_condition_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> fmt::Result {
        self.visit_expression(left)?;
        self.write(" = ")?;
        self.visit_expression(right)
    }

    #[cfg(feature = "json-1")]
    fn visit_condition_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> fmt::Result {
        let (left_is_json, right_is_json) = (left.is_json_value(), right.is_json_value());

        self.visit_expression(left)?;

        if right_is_json {
            self.write("::jsonb")?;
        }

        self.write(" <> ")?;

        if left_is_json {
            self.write("::jsonb")?;
        }

        self.visit_expression(right)
    }

    #[cfg(not(feature = "json-1"))]
    fn visit_condition_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> fmt::Result {
        self.visit_expression(left)?;
        self.write(" <> ")?;
        self.visit_expression(right)
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
        let (sql, params) = Postgres::build(query);

        assert_eq!("INSERT INTO \"users\" DEFAULT VALUES", sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_single_row_insert() {
        let expected = expected_values("INSERT INTO \"users\" (\"foo\") VALUES ($1)", vec![10]);
        let query = Insert::single_into("users").value("foo", 10);
        let (sql, params) = Postgres::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_multi_row_insert() {
        let expected = expected_values("INSERT INTO \"users\" (\"foo\") VALUES ($1), ($2)", vec![10, 11]);
        let query = Insert::multi_into("users", vec!["foo"])
            .values(vec![10])
            .values(vec![11]);
        let (sql, params) = Postgres::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_both_are_set() {
        let expected = expected_values("SELECT \"users\".* FROM \"users\" LIMIT $1 OFFSET $2", vec![10, 2]);
        let query = Select::from_table("users").limit(10).offset(2);
        let (sql, params) = Postgres::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_offset_is_set() {
        let expected = expected_values("SELECT \"users\".* FROM \"users\" OFFSET $1", vec![10]);
        let query = Select::from_table("users").offset(10);
        let (sql, params) = Postgres::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_limit_is_set() {
        let expected = expected_values("SELECT \"users\".* FROM \"users\" LIMIT $1", vec![10]);
        let query = Select::from_table("users").limit(10);
        let (sql, params) = Postgres::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[cfg(feature = "json-1")]
    #[test]
    fn equality_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "jsonField"::jsonb = $1"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let query = Select::from_table("users").so_that(Column::from("jsonField").equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Postgres::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[cfg(feature = "json-1")]
    #[test]
    fn difference_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "jsonField"::jsonb <> $1"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let query =
            Select::from_table("users").so_that(Column::from("jsonField").not_equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Postgres::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }
}
