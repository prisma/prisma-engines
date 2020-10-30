//! Convert results from the database into any type implementing `serde::Deserialize`.

use std::borrow::Cow;

use crate::{
    ast::Value,
    connector::{ResultRow, ResultSet},
    error::{Error, ErrorKind},
};
use serde::{de::Error as SerdeError, de::*};

impl ResultSet {
    /// Takes the first row and deserializes it.
    pub fn from_first<T: DeserializeOwned>(self) -> crate::Result<T> {
        Ok(from_row(self.into_single()?)?)
    }
}

/// Deserialize each row of a [`ResultSet`](../connector/struct.ResultSet.html).
///
/// For an example, see the docs for [`from_row`](fn.from_row.html).
pub fn from_rows<T: DeserializeOwned>(result_set: ResultSet) -> crate::Result<Vec<T>> {
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
/// # use quaint::ast::Value;
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
/// #       ("id", Value::from(12)),
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
pub fn from_row<T: DeserializeOwned>(row: ResultRow) -> crate::Result<T> {
    let deserializer = RowDeserializer(row);

    T::deserialize(deserializer).map_err(|e| Error::builder(ErrorKind::FromRowError(e)).build())
}

type DeserializeError = serde::de::value::Error;

#[derive(Debug)]
struct RowDeserializer(ResultRow);

impl<'de> Deserializer<'de> for RowDeserializer {
    type Error = DeserializeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let ResultRow { columns, mut values } = self.0;

        let kvs = columns.iter().enumerate().map(move |(v, k)| {
            // The unwrap is safe if `columns` is correct.
            let value = values.get_mut(v).unwrap();
            let taken_value = std::mem::replace(value, Value::Integer(None));
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

impl<'de> IntoDeserializer<'de, DeserializeError> for Value<'de> {
    type Deserializer = ValueDeserializer<'de>;

    fn into_deserializer(self) -> Self::Deserializer {
        ValueDeserializer(self)
    }
}

#[derive(Debug)]
pub struct ValueDeserializer<'a>(Value<'a>);

impl<'de> Deserializer<'de> for ValueDeserializer<'de> {
    type Error = DeserializeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Text(Some(s)) => visitor.visit_string(s.into_owned()),
            Value::Text(None) => visitor.visit_none(),
            Value::Bytes(Some(bytes)) => visitor.visit_bytes(bytes.as_ref()),
            Value::Bytes(None) => visitor.visit_none(),
            Value::Enum(Some(s)) => visitor.visit_string(s.into_owned()),
            Value::Enum(None) => visitor.visit_none(),
            Value::Integer(Some(i)) => visitor.visit_i64(i),
            Value::Integer(None) => visitor.visit_none(),
            Value::Boolean(Some(b)) => visitor.visit_bool(b),
            Value::Boolean(None) => visitor.visit_none(),
            Value::Char(Some(c)) => visitor.visit_char(c),
            Value::Char(None) => visitor.visit_none(),
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(Some(num)) => visitor.visit_f64(num.to_string().parse::<f64>().unwrap()),
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(None) => visitor.visit_none(),

            #[cfg(feature = "uuid")]
            Value::Uuid(Some(uuid)) => visitor.visit_string(uuid.to_string()),
            #[cfg(feature = "uuid")]
            Value::Uuid(None) => visitor.visit_none(),

            #[cfg(feature = "json")]
            Value::Json(Some(value)) => {
                let de = value.into_deserializer();

                de.deserialize_any(visitor)
                    .map_err(|err| serde::de::value::Error::custom(format!("Error deserializing JSON value: {}", err)))
            }
            #[cfg(feature = "json")]
            Value::Json(None) => visitor.visit_none(),

            Value::Xml(Some(s)) => visitor.visit_string(s.into_owned()),
            Value::Xml(None) => visitor.visit_none(),

            #[cfg(feature = "chrono")]
            Value::DateTime(Some(dt)) => visitor.visit_string(dt.to_rfc3339()),
            #[cfg(feature = "chrono")]
            Value::DateTime(None) => visitor.visit_none(),

            #[cfg(feature = "chrono")]
            Value::Date(Some(d)) => visitor.visit_string(format!("{}", d)),
            #[cfg(feature = "chrono")]
            Value::Date(None) => visitor.visit_none(),

            #[cfg(feature = "chrono")]
            Value::Time(Some(t)) => visitor.visit_string(format!("{}", t)),
            #[cfg(feature = "chrono")]
            Value::Time(None) => visitor.visit_none(),

            Value::Array(Some(values)) => {
                let deserializer = serde::de::value::SeqDeserializer::new(values.into_iter());
                visitor.visit_seq(deserializer)
            }
            Value::Array(None) => visitor.visit_none(),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.0.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Value::Bytes(Some(bytes)) = self.0 {
            match bytes {
                Cow::Borrowed(bytes) => visitor.visit_borrowed_bytes(bytes),
                Cow::Owned(bytes) => visitor.visit_byte_buf(bytes),
            }
        } else {
            Err(DeserializeError::invalid_type(
                Unexpected::Other(&format!("{:?}", self.0)),
                &visitor,
            ))
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str byte_buf
        string unit unit_struct newtype_struct seq tuple tuple_struct map
        struct enum identifier ignored_any
    }
}

#[doc(hidden)]
pub fn make_row(cols: Vec<(&'static str, Value<'static>)>) -> ResultRow {
    let mut columns = Vec::with_capacity(cols.len());
    let mut values = Vec::with_capacity(cols.len());

    for (name, value) in cols.into_iter() {
        columns.push(name.to_owned());
        values.push(value);
    }

    ResultRow {
        values,
        columns: std::sync::Arc::new(columns),
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
        bio: Option<String>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Cat {
        age: f32,
        birthday: DateTime<Utc>,
        human: User,
    }

    #[test]
    fn deserialize_user() {
        let row = make_row(vec![("id", Value::integer(12)), ("name", "Georgina".into())]);
        let user: User = from_row(row).unwrap();

        assert_eq!(
            user,
            User {
                id: 12,
                name: "Georgina".to_owned(),
                bio: None,
            }
        )
    }

    #[test]
    fn from_rows_works() {
        let first_row = make_row(vec![
            ("id", Value::integer(12)),
            ("name", "Georgina".into()),
            ("bio", Value::Text(None)),
        ]);
        let second_row = make_row(vec![
            ("id", 33.into()),
            ("name", "Philbert".into()),
            (
                "bio",
                "Invented sliced bread on a meditation retreat in the Himalayas.".into(),
            ),
        ]);

        let result_set = ResultSet {
            columns: std::sync::Arc::clone(&first_row.columns),
            rows: vec![first_row.values, second_row.values],
            last_insert_id: None,
        };

        let users: Vec<User> = from_rows(result_set).unwrap();

        assert_eq!(
            users,
            &[
                User {
                    id: 12,
                    name: "Georgina".to_owned(),
                    bio: None,
                },
                User {
                    id: 33,
                    name: "Philbert".to_owned(),
                    bio: Some("Invented sliced bread on a meditation retreat in the Himalayas.".into()),
                }
            ]
        );
    }

    #[test]
    fn deserialize_cat() {
        let row = make_row(vec![
            ("age", Value::numeric("18.800001".parse().unwrap())),
            ("birthday", Value::datetime("2019-08-01T20:00:00Z".parse().unwrap())),
            (
                "human",
                Value::json(serde_json::json!({
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
                bio: None,
            },
        };

        assert_eq!(cat, expected_cat);
    }
}
