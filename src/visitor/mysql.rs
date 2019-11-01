use crate::{ast::*, visitor::Visitor};
use mysql_async::Value as MyValue;
use std::fmt::{self, Write};

#[cfg(feature = "chrono-0_4")]
use chrono::{Datelike, Timelike};

/// A visitor to generate queries for the MySQL database.
///
/// The returned parameter values can be used directly with the mysql crate.
pub struct Mysql<'a> {
    query: String,
    parameters: Vec<ParameterizedValue<'a>>,
}

impl<'a> Visitor<'a> for Mysql<'a> {
    const C_BACKTICK: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> (String, Vec<ParameterizedValue<'a>>)
    where
        Q: Into<Query<'a>>,
    {
        let mut mysql = Mysql {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(1024),
        };

        Mysql::visit_query(&mut mysql, query.into());

        (
            mysql.query,
            mysql.parameters,
        )
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

    fn add_parameter(&mut self, value: ParameterizedValue<'a>) {
        self.parameters.push(value);
    }

    fn visit_limit_and_offset(
        &mut self,
        limit: Option<ParameterizedValue<'a>>,
        offset: Option<ParameterizedValue<'a>>,
    ) -> fmt::Result {
        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            },
            (None, Some(ParameterizedValue::Integer(offset))) if offset < 1 => Ok(()),
            (None, Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(ParameterizedValue::from(9_223_372_036_854_775_807i64))?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            },
            (Some(limit), None) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)
            }
            (None, None) => Ok(()),
        }
    }

    fn visit_aggregate_to_string(&mut self, value: DatabaseValue<'a>) -> fmt::Result {
        self.write(" GROUP_CONCAT")?;
        self.surround_with("(", ")", |ref mut s| s.visit_database_value(value))
    }
}

impl<'a> From<ParameterizedValue<'a>> for MyValue {
    fn from(pv: ParameterizedValue<'a>) -> MyValue {
        match pv {
            ParameterizedValue::Null => MyValue::NULL,
            ParameterizedValue::Integer(i) => MyValue::Int(i),
            ParameterizedValue::Real(f) => MyValue::Float(f),
            ParameterizedValue::Text(s) => MyValue::Bytes((&*s).as_bytes().to_vec()),
            ParameterizedValue::Boolean(b) => MyValue::Int(b as i64),
            ParameterizedValue::Char(c) => MyValue::Bytes(vec![c as u8]),
            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(json) => {
                let s = serde_json::to_string(&json).expect("Cannot convert JSON to String.");

                MyValue::Bytes(s.into_bytes())
            }
            #[cfg(feature = "array")]
            ParameterizedValue::Array(_) => unimplemented!("Arrays are not supported for mysql."),
            #[cfg(feature = "uuid-0_7")]
            ParameterizedValue::Uuid(u) => {
                MyValue::Bytes(u.to_hyphenated().to_string().into_bytes())
            }
            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(dt) => MyValue::Date(
                dt.year() as u16,
                dt.month() as u8,
                dt.day() as u8,
                dt.hour() as u8,
                dt.minute() as u8,
                dt.second() as u8,
                dt.timestamp_subsec_micros(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::visitor::*;

    fn expected_values<'a, T>(
        sql: &'static str,
        params: Vec<T>,
    ) -> (String, Vec<ParameterizedValue<'a>>)
    where
        T: Into<ParameterizedValue<'a>>,
    {
        (
            String::from(sql),
            params.into_iter().map(|p| p.into()).collect(),
        )
    }

    fn default_params<'a>(
        mut additional: Vec<ParameterizedValue<'a>>,
    ) -> Vec<ParameterizedValue<'a>> {
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
        let query = Insert::multi_into("users", vec!["foo"]).values(vec![10]).values(vec![11]);
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
            vec![9_223_372_036_854_775_807i64,10]
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
}
