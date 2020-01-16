//! Convert results from the database into any type implementing `serde::Deserialize`.

use crate::{
    ast::ParameterizedValue,
    connector::{ResultRow, ResultSet},
};
use serde::{de::Error as SerdeError, de::*};
use thiserror::Error;

/// Deserialize each row of a [`ResultSet`](../connector/struct.ResultSet.html).
///
/// For an example, see the docs for [`from_row`](fn.from_row.html).
pub fn from_rows<T: DeserializeOwned>(result_set: ResultSet) -> Result<Vec<T>, FromRowError> {
    let mut deserialized_rows = Vec::with_capacity(result_set.len());

    for row in result_set {
        deserialized_rows.push(from_row(row)?)
    }

    Ok(deserialized_rows)
}

/// Deserialize a row into any type implementing `Deserialize`.
///
/// ```
/// # use serde::Deserialize;
/// # use quaint::ast::ParameterizedValue;
/// #
/// # #[derive(Deserialize, Debug, PartialEq)]
/// # struct User {
/// #     id: u64,
/// #     name: String,
/// # }
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #
/// #   let row = quaint::serde::make_row(vec![
/// #       ("id", ParameterizedValue::Integer(12)),
/// #       ("name", "Georgina".into()),
/// #   ]);
/// #
/// #
/// let user: User = quaint::serde::from_row(row)?;
///
/// assert_eq!(user, User { name: "Georgina".to_string(), id: 12 });
/// # Ok(())
/// # }
/// ```
pub fn from_row<T: DeserializeOwned>(row: ResultRow) -> Result<T, FromRowError> {
    let deserializer = RowDeserializer(row);

    T::deserialize(deserializer).map_err(FromRowError)
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct FromRowError(DeserializeError);

type DeserializeError = serde::de::value::Error;

#[derive(Debug)]
struct RowDeserializer(ResultRow);

impl<'de> Deserializer<'de> for RowDeserializer {
    type Error = DeserializeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let ResultRow {
            name_to_index,
            mut values,
        } = self.0;

        let kvs = name_to_index.iter().map(move |(k, v)| {
            // The unwrap is safe if `name_to_index` is correct.
            let value = values.get_mut(*v).unwrap();
            let taken_value = std::mem::replace(value, ParameterizedValue::Null);
            (k.as_str(), taken_value)
        });

        let deserializer = serde::de::value::MapDeserializer::new(kvs);

        visitor.visit_map(deserializer)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map
        struct enum identifier ignored_any
    }
}

impl<'de> IntoDeserializer<'de, DeserializeError> for ParameterizedValue<'de> {
    type Deserializer = ParameterizedValueDeserializer<'de>;

    fn into_deserializer(self) -> Self::Deserializer {
        ParameterizedValueDeserializer(self)
    }
}

#[derive(Debug)]
pub struct ParameterizedValueDeserializer<'a>(ParameterizedValue<'a>);

impl<'de> Deserializer<'de> for ParameterizedValueDeserializer<'de> {
    type Error = DeserializeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        use rust_decimal::prelude::ToPrimitive;

        match self.0 {
            ParameterizedValue::Text(s) => visitor.visit_string(s.into_owned()),
            ParameterizedValue::Integer(i) => visitor.visit_i64(i),
            ParameterizedValue::Boolean(b) => visitor.visit_bool(b),
            ParameterizedValue::Char(c) => visitor.visit_char(c),
            ParameterizedValue::Null => visitor.visit_none(),
            ParameterizedValue::Real(real) => visitor.visit_f64(real.to_f64().unwrap()),

            #[cfg(feature = "uuid-0_8")]
            ParameterizedValue::Uuid(uuid) => visitor.visit_string(uuid.to_string()),

            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(value) => value
                .into_deserializer()
                .deserialize_any(visitor)
                .map_err(|err| serde::de::value::Error::custom(format!("Error deserializing JSON value: {}", err))),

            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(dt) => visitor.visit_string(dt.to_rfc3339()),

            #[cfg(all(feature = "array", feature = "postgresql"))]
            ParameterizedValue::Array(values) => {
                let deserializer = serde::de::value::SeqDeserializer::new(values.into_iter());
                visitor.visit_seq(deserializer)
            }
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map
        struct enum identifier ignored_any
    }
}

#[doc(hidden)]
pub fn make_row(cols: Vec<(&'static str, ParameterizedValue<'static>)>) -> ResultRow {
    use std::collections::BTreeMap;

    let mut name_to_index = BTreeMap::new();
    let mut values = Vec::with_capacity(cols.len());

    for (idx, (name, value)) in cols.into_iter().enumerate() {
        name_to_index.insert(name.to_owned(), idx);
        values.push(value);
    }

    ResultRow {
        values,
        name_to_index: std::sync::Arc::new(name_to_index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct User {
        id: u64,
        name: String,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Cat {
        age: f32,
        birthday: DateTime<Utc>,
        human: User,
    }

    #[test]
    fn deserialize_user() {
        let row = make_row(vec![
            ("id", ParameterizedValue::Integer(12)),
            ("name", "Georgina".into()),
        ]);
        let user: User = from_row(row).unwrap();

        assert_eq!(
            user,
            User {
                id: 12,
                name: "Georgina".to_owned()
            }
        )
    }

    #[test]
    fn from_rows_works() {
        let first_row = make_row(vec![
            ("id", ParameterizedValue::Integer(12)),
            ("name", "Georgina".into()),
        ]);
        let second_row = make_row(vec![("id", 33.into()), ("name", "Philbert".into())]);

        let result_set = ResultSet {
            name_to_index: std::sync::Arc::clone(&first_row.name_to_index),
            rows: vec![first_row.values, second_row.values],
        };

        let users: Vec<User> = from_rows(result_set).unwrap();

        assert_eq!(
            users,
            &[
                User {
                    id: 12,
                    name: "Georgina".to_owned(),
                },
                User {
                    id: 33,
                    name: "Philbert".to_owned(),
                }
            ]
        );
    }

    #[test]
    fn deserialize_cat() {
        let row = make_row(vec![
            ("age", ParameterizedValue::Real("18.800001".parse().unwrap())),
            (
                "birthday",
                ParameterizedValue::DateTime("2019-08-01T20:00:00Z".parse().unwrap()),
            ),
            (
                "human",
                ParameterizedValue::Json(serde_json::json!({
                    "id": 19,
                    "name": "Georgina"
                })),
            ),
        ]);
        let cat: Cat = from_row(row).unwrap();

        let expected_cat = Cat {
            age: 18.800001,
            birthday: "2019-08-01T20:00:00Z".parse().unwrap(),
            human: User {
                name: "Georgina".into(),
                id: 19,
            },
        };

        assert_eq!(cat, expected_cat);
    }
}
