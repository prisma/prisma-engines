use quaint::{
    connector::{ColumnType, ResultRowRef, ResultSet},
    Value, ValueType,
};
use serde::{ser::*, Serialize, Serializer};

pub struct SerializedResultSet(pub ResultSet);

#[derive(Debug, Serialize)]
struct InnerSerializedResultSet<'a> {
    columns: SerializedColumns<'a>,
    types: SerializedTypes<'a>,
    rows: SerializedRows<'a>,
}

impl serde::Serialize for SerializedResultSet {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let this = &self.0;

        InnerSerializedResultSet {
            columns: SerializedColumns(this),
            types: SerializedTypes(this),
            rows: SerializedRows(this),
        }
        .serialize(serializer)
    }
}

#[derive(Debug)]
struct SerializedColumns<'a>(&'a ResultSet);

impl Serialize for SerializedColumns<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let this = &self.0;

        this.columns().serialize(serializer)
    }
}

#[derive(Debug)]
struct SerializedTypes<'a>(&'a ResultSet);

impl SerializedTypes<'_> {
    fn infer_unknown_column_types(&self) -> Vec<ColumnType> {
        let rows = self.0;

        let mut types = rows.types().to_owned();
        // Find all the unknown column types to avoid unnecessary iterations.
        let unknown_indexes = rows
            .types()
            .iter()
            .enumerate()
            .filter_map(|(idx, ty)| match ty.is_unknown() {
                true => Some(idx),
                false => None,
            });

        for unknown_idx in unknown_indexes {
            // While quaint already infers `ColumnType`s from the database, it can still have ColumnType::Unknown.
            // In this case, we try to infer the types from the actual response data.
            for row in self.0.iter() {
                let current_type = types[unknown_idx];
                let inferred_type = ColumnType::from(&row[unknown_idx]);

                if current_type.is_unknown() && !inferred_type.is_unknown() {
                    types[unknown_idx] = inferred_type;
                    break;
                }
            }
        }

        if !self.0.is_empty() {
            // Client doesn't know how to handle unknown types.
            assert!(!types.contains(&ColumnType::Unknown));
        }

        types
    }
}

impl Serialize for SerializedTypes<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let types = self.infer_unknown_column_types();

        let mut seq = serializer.serialize_seq(Some(types.len()))?;

        for column_type in types {
            seq.serialize_element(&column_type.to_string())?;
        }

        seq.end()
    }
}

#[derive(Debug)]
struct SerializedRows<'a>(&'a ResultSet);

impl Serialize for SerializedRows<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let rows = &self.0;
        let mut seq = serializer.serialize_seq(Some(rows.len()))?;

        for row in rows.iter() {
            seq.serialize_element(&SerializedRow(&row))?;
        }

        seq.end()
    }
}

struct SerializedRow<'a>(&'a ResultRowRef<'a>);

impl Serialize for SerializedRow<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let row = &self.0;
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for value in row.iter() {
            seq.serialize_element(&SerializedValue(value))?;
        }

        seq.end()
    }
}

struct SerializedValue<'a>(&'a Value<'a>);

impl Serialize for SerializedValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val = &self.0;

        match &val.typed {
            ValueType::Array(Some(values)) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;

                for value in values {
                    seq.serialize_element(&SerializedValue(value))?;
                }

                seq.end()
            }
            ValueType::Array(None) => serializer.serialize_none(),
            ValueType::Int32(value) => value.serialize(serializer),
            ValueType::Int64(value) => value.map(|val| val.to_string()).serialize(serializer),
            ValueType::Numeric(value) => value
                .as_ref()
                .map(|value| value.normalized().to_string())
                .serialize(serializer),
            ValueType::Float(value) => value.serialize(serializer),
            ValueType::Double(value) => value.serialize(serializer),
            ValueType::Text(value) => value.serialize(serializer),
            ValueType::Enum(value, _) => value.as_ref().map(|value| value.inner()).serialize(serializer),
            ValueType::EnumArray(Some(variants), _) => {
                let mut seq = serializer.serialize_seq(Some(variants.len()))?;

                for variant in variants {
                    seq.serialize_element(variant.inner())?;
                }

                seq.end()
            }
            ValueType::EnumArray(None, _) => serializer.serialize_none(),
            ValueType::Bytes(value) => value.as_ref().map(prisma_value::encode_base64).serialize(serializer),
            ValueType::Boolean(value) => value.serialize(serializer),
            ValueType::Char(value) => value.serialize(serializer),
            ValueType::Json(value) => value.serialize(serializer),
            ValueType::Xml(value) => value.serialize(serializer),
            ValueType::Uuid(value) => value.serialize(serializer),
            ValueType::DateTime(value) => value.map(|value| value.to_rfc3339()).serialize(serializer),
            ValueType::Date(value) => value.serialize(serializer),
            ValueType::Time(value) => value.serialize(serializer),
            ValueType::Var(_, _) => unreachable!(),
        }
    }
}
