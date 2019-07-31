use crate::{ast::*, visitor::Visitor};
use mysql::Value as MyValue;

#[cfg(feature = "chrono-0_4")]
use chrono::{Datelike, Timelike};

/// A visitor to generate queries for the MySQL database.
///
/// The returned parameter values can be used directly with the mysql crate.
pub struct Mysql<'a> {
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
            parameters: Vec::new(),
        };

        let result = (
            Mysql::visit_query(&mut mysql, query.into()),
            mysql.parameters,
        );

        debug!("query: \"{}\", params: [{}]", result.0, Params(result.1.as_slice()));

        result
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> String {
        let mut result = match insert.on_conflict {
            Some(OnConflict::DoNothing) => vec![String::from("INSERT IGNORE")],
            None => vec![String::from("INSERT")],
        };

        result.push(format!("INTO {}", self.visit_table(insert.table, true)));

        if insert.values.is_empty() {
            result.push("() VALUES ()".to_string());
        } else {
            let columns: Vec<String> = insert
                .columns
                .into_iter()
                .map(|c| self.visit_column(Column::from(c)))
                .collect();

            let values: Vec<String> = insert
                .values
                .into_iter()
                .map(|row| self.visit_row(row))
                .collect();

            result.push(format!(
                "({}) VALUES {}",
                columns.join(", "),
                values.join(", "),
            ))
        }

        result.join(" ")
    }

    fn parameter_substitution(&self) -> String {
        String::from("?")
    }

    fn add_parameter(&mut self, value: ParameterizedValue<'a>) {
        self.parameters.push(value);
    }

    fn visit_limit_and_offset(
        &mut self,
        limit: Option<ParameterizedValue<'a>>,
        offset: Option<ParameterizedValue<'a>>,
    ) -> Option<String> {
        match (limit, offset) {
            (Some(limit), Some(offset)) => Some(format!(
                "LIMIT {} OFFSET {}",
                self.visit_parameterized(limit),
                self.visit_parameterized(offset)
            )),
            (None, Some(ParameterizedValue::Integer(offset))) if offset < 1 => None,
            (None, Some(offset)) => Some(format!(
                "LIMIT {} OFFSET {}",
                self.visit_parameterized(ParameterizedValue::from(9223372036854775807i64)),
                self.visit_parameterized(offset),
            )),
            (Some(limit), None) => Some(format!("LIMIT {}", self.visit_parameterized(limit))),
            (None, None) => None,
        }
    }

    fn visit_aggregate_to_string(&mut self, value: DatabaseValue<'a>) -> String {
        format!("group_concat({})", self.visit_database_value(value))
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
