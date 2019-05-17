use crate::{ast::*, visitor::Visitor};
use mysql::Value as MyValue;

#[cfg(feature = "chrono-0_4")]
use chrono::{Datelike, Timelike};

pub struct Mysql {
    parameters: Vec<ParameterizedValue>,
}

impl Visitor for Mysql {
    const C_BACKTICK: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> (String, Vec<ParameterizedValue>)
    where
        Q: Into<Query>,
    {
        let mut mysql = Mysql {
            parameters: Vec::new(),
        };

        (
            Mysql::visit_query(&mut mysql, query.into()),
            mysql.parameters,
        )
    }

    fn visit_insert(&mut self, insert: Insert) -> String {
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

    fn add_parameter(&mut self, value: ParameterizedValue) {
        self.parameters.push(value);
    }

    fn visit_limit_and_offset(
        &mut self,
        limit: Option<ParameterizedValue>,
        offset: Option<ParameterizedValue>,
    ) -> Option<String> {
        match (limit, offset) {
            (Some(limit), Some(offset)) => Some(format!(
                "LIMIT {} OFFSET {}",
                self.visit_parameterized(limit),
                self.visit_parameterized(offset)
            )),
            (None, Some(offset)) => Some(format!(
                "LIMIT {} OFFSET {}",
                self.visit_parameterized(ParameterizedValue::from(9223372036854775807i64)),
                self.visit_parameterized(offset)
            )),
            (Some(limit), None) => Some(format!("LIMIT {}", self.visit_parameterized(limit))),
            (None, None) => None,
        }
    }
}

impl From<ParameterizedValue> for MyValue {
    fn from(pv: ParameterizedValue) -> MyValue {
        match pv {
            ParameterizedValue::Null => MyValue::NULL,
            ParameterizedValue::Integer(i) => MyValue::Int(i),
            ParameterizedValue::Real(f) => MyValue::Float(f),
            ParameterizedValue::Text(s) => MyValue::Bytes(s.into_bytes()),
            ParameterizedValue::Boolean(b) => MyValue::Int(b as i64),
            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(json) => {
                let s = serde_json::to_string(&json).expect("Cannot convert JSON to String.");

                MyValue::Bytes(s.into_bytes())
            }
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
