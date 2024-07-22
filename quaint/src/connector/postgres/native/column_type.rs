use crate::connector::ColumnType;

use std::borrow::Cow;
use tokio_postgres::types::{Kind as PostgresKind, Type as PostgresType};

macro_rules! create_pg_mapping {
  (
    $($key:ident($typ: ty) => [$($value:ident),+]),* $(,)?
    $([$pg_only_key:ident => $column_type_mapping:ident]),*
  ) => {
      // Generate PGColumnType<Type> enums
      $(
          concat_idents::concat_idents!(enum_name = PGColumnType, $key {
            #[derive(Debug)]
            #[allow(non_camel_case_types)]
            #[allow(clippy::upper_case_acronyms)]
            pub(crate) enum enum_name {
                $($value,)*
            }
        });
      )*

      // Generate validators
      $(
        concat_idents::concat_idents!(struct_name = PGColumnValidator, $key {
            #[derive(Debug)]
            #[allow(non_camel_case_types)]
            pub struct struct_name;

            impl struct_name {
                #[inline]
                #[allow(clippy::extra_unused_lifetimes)]
                pub fn read<'a>(&self, val: $typ) -> $typ {
                    val
                }
            }
        });
    )*

      pub(crate) enum PGColumnType {
        $(
          $key(
            concat_idents::concat_idents!(variant = PGColumnType, $key, { variant }),
            concat_idents::concat_idents!(enum_name = PGColumnValidator, $key, { enum_name })
          ),
        )*
        $($pg_only_key(concat_idents::concat_idents!(enum_name = PGColumnValidator, $column_type_mapping, { enum_name })),)*
      }

      impl PGColumnType {
          /// Takes a Postgres type and returns the corresponding ColumnType
          #[deny(unreachable_patterns)]
          pub(crate) fn from_pg_type(ty: &PostgresType) -> PGColumnType {
              match ty {
                  $(
                      $(
                        &PostgresType::$value => PGColumnType::$key(
                          concat_idents::concat_idents!(variant = PGColumnType, $key, { variant::$value }),
                          concat_idents::concat_idents!(enum_name = PGColumnValidator, $key, { enum_name }),
                        ),
                      )*
                  )*
                  ref x => match x.kind() {
                      PostgresKind::Enum => PGColumnType::Enum(PGColumnValidatorText),
                      PostgresKind::Array(inner) => match inner.kind() {
                          PostgresKind::Enum => PGColumnType::EnumArray(PGColumnValidatorTextArray),
                          _ => PGColumnType::UnknownArray(PGColumnValidatorTextArray),
                      },
                      _ => PGColumnType::Unknown(PGColumnValidatorText),
                  },
              }
          }
      }

      impl From<PGColumnType> for ColumnType {
          fn from(ty: PGColumnType) -> ColumnType {
              match ty {
                  $(
                      PGColumnType::$key(..) => ColumnType::$key,
                  )*
                  $(
                      PGColumnType::$pg_only_key(..) => ColumnType::$column_type_mapping,
                  )*
              }
          }
      }
  };
}

// Create a mapping between Postgres types and ColumnType and ensures there's a single source of truth.
// ColumnType(<accepted data>) => [PostgresType(s)...]
create_pg_mapping! {
  Boolean(Option<bool>) => [BOOL],
  Int32(Option<i32>) => [INT2, INT4],
  Int64(Option<i64>) => [INT8, OID],
  Float(Option<f32>) => [FLOAT4],
  Double(Option<f64>) => [FLOAT8],
  Bytes(Option<Cow<'a, [u8]>>) => [BYTEA],
  Numeric(Option<bigdecimal::BigDecimal>) => [NUMERIC, MONEY],
  DateTime(Option<chrono::DateTime<chrono::Utc>>) => [TIMESTAMP, TIMESTAMPTZ],
  Date(Option<chrono::NaiveDate>) => [DATE],
  Time(Option<chrono::NaiveTime>) => [TIME, TIMETZ],
  Text(Option<Cow<'a, str>>) => [INET, CIDR, BIT, VARBIT],
  Uuid(Option<uuid::Uuid>) => [UUID],
  Json(Option<serde_json::Value>) => [JSON, JSONB],
  Xml(Option<Cow<'a, str>>) => [XML],
  Char(Option<char>) => [CHAR],

  BooleanArray(impl Iterator<Item = Option<bool>>) => [BOOL_ARRAY],
  Int32Array(impl Iterator<Item = Option<i32>>) => [INT2_ARRAY, INT4_ARRAY],
  Int64Array(impl Iterator<Item =Option<i64>>) => [INT8_ARRAY, OID_ARRAY],
  FloatArray(impl Iterator<Item = Option<f32>>) => [FLOAT4_ARRAY],
  DoubleArray(impl Iterator<Item = Option<f64>>) => [FLOAT8_ARRAY],
  BytesArray(impl Iterator<Item = Option<Vec<u8>>>) => [BYTEA_ARRAY],
  NumericArray(impl Iterator<Item = Option<bigdecimal::BigDecimal>>) => [NUMERIC_ARRAY, MONEY_ARRAY],
  DateTimeArray(impl Iterator<Item = Option<chrono::DateTime<chrono::Utc>>>) => [TIMESTAMP_ARRAY, TIMESTAMPTZ_ARRAY],
  DateArray(impl Iterator<Item = Option<chrono::NaiveDate>>) => [DATE_ARRAY],
  TimeArray(impl Iterator<Item = Option<chrono::NaiveTime>>) => [TIME_ARRAY, TIMETZ_ARRAY],
  TextArray(impl Iterator<Item = Option<Cow<'a, str>>>) => [TEXT_ARRAY, NAME_ARRAY, VARCHAR_ARRAY, INET_ARRAY, CIDR_ARRAY, BIT_ARRAY, VARBIT_ARRAY, XML_ARRAY],
  UuidArray(impl Iterator<Item = Option<uuid::Uuid>>) => [UUID_ARRAY],
  JsonArray(impl Iterator<Item = Option<serde_json::Value>>) => [JSON_ARRAY, JSONB_ARRAY],

  // For the cases where the Postgres type is not directly mappable to ColumnType, use the following:
  // [PGColumnType => ColumnType]
  [Enum => Text],
  [EnumArray => TextArray],
  [UnknownArray => TextArray],
  [Unknown => Text]
}
