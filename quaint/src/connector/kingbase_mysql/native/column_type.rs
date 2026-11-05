use crate::{
    connector::ColumnType,
    error::{Error, ErrorKind},
};

use kingbase_tokio_postgres::{
    Column as KingbaseColumn,
    types::{Kind as KingbaseKind, Type as KingbaseType},
};
use std::borrow::Cow;

/// SQL arrays are not part of the MySQL connector's supported value types.
/// Check metadata as well as values so NULL arrays and empty result sets fail.
pub(crate) fn ensure_supported_type(typ: &KingbaseType) -> crate::Result<()> {
    match typ.kind() {
        KingbaseKind::Array(_) => Err(Error::builder(ErrorKind::UnsupportedColumnType {
            column_type: typ.to_string(),
        })
        .build()),
        KingbaseKind::Domain(inner) => ensure_supported_type(inner),
        _ => Ok(()),
    }
}

/// Kingbase's MySQL-compatible BIT type stores the declared bit width directly
/// in typmod. A negative typmod means that no fixed length was declared.
pub(crate) fn mysql_bit_length_from_type_modifier(type_modifier: i32) -> Option<u64> {
    u64::try_from(type_modifier).ok().filter(|length| *length > 0)
}

pub(crate) fn is_mysql_bit_one(column: &KingbaseColumn) -> bool {
    column.type_() == &KingbaseType::MYSQL_SYS_BIT
        && mysql_bit_length_from_type_modifier(column.type_modifier()) == Some(1)
}

pub(crate) fn column_type_from_column(column: &KingbaseColumn) -> ColumnType {
    if is_mysql_bit_one(column) {
        ColumnType::Boolean
    } else {
        ColumnType::from(column.type_())
    }
}

macro_rules! create_pg_mapping {
  (
    $($key:ident($typ: ty) => [$($value:ident),+]),* $(,)?
    $([$pg_only_key:ident => $column_type_mapping:ident]),*
  ) => {
      // Generate KingbaseColumnType<Type> enums
      $(
          concat_idents::concat_idents!(enum_name = KingbaseColumnType, $key {
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
        concat_idents::concat_idents!(struct_name = KingbaseColumnValidator, $key {
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

      pub(crate) enum KingbaseColumnType {
        $(
          $key(
            concat_idents::concat_idents!(variant = KingbaseColumnType, $key, { variant }),
            concat_idents::concat_idents!(enum_name = KingbaseColumnValidator, $key, { enum_name })
          ),
        )*
        $($pg_only_key(concat_idents::concat_idents!(enum_name = KingbaseColumnValidator, $column_type_mapping, { enum_name })),)*
      }

      impl KingbaseColumnType {
          /// Takes a Postgres type and returns the corresponding ColumnType
          #[deny(unreachable_patterns)]
          pub(crate) fn from_pg_type(ty: &KingbaseType) -> KingbaseColumnType {
              match ty {
                  $(
                      $(
                        &KingbaseType::$value => KingbaseColumnType::$key(
                          concat_idents::concat_idents!(variant = KingbaseColumnType, $key, { variant::$value }),
                          concat_idents::concat_idents!(enum_name = KingbaseColumnValidator, $key, { enum_name }),
                        ),
                      )*
                  )*
                  ref x => match x.kind() {
                      KingbaseKind::Enum(_) | KingbaseKind::MySqlEnum(_) => {
                          KingbaseColumnType::Enum(KingbaseColumnValidatorText)
                      }
                      KingbaseKind::MySqlSet => KingbaseColumnType::Set(KingbaseColumnValidatorText),
                      _ => KingbaseColumnType::Unknown(KingbaseColumnValidatorText),
                  },
              }
          }
      }

      impl From<KingbaseColumnType> for ColumnType {
          fn from(ty: KingbaseColumnType) -> ColumnType {
              match ty {
                  $(
                      KingbaseColumnType::$key(..) => ColumnType::$key,
                  )*
                  $(
                      KingbaseColumnType::$pg_only_key(..) => ColumnType::$column_type_mapping,
                  )*
              }
          }
      }

      impl From<&KingbaseType> for ColumnType {
          fn from(ty: &KingbaseType) -> ColumnType {
              // MySQL ENUMs are exposed as a first-class enum by the MySQL
              // connector. Keep PostgreSQL enums on their existing Text path.
              if matches!(ty.kind(), KingbaseKind::MySqlEnum(_)) {
                  ColumnType::Enum
              } else {
                  KingbaseColumnType::from_pg_type(&ty).into()
              }
          }
      }
  };
}

// Create a mapping between Postgres types and ColumnType and ensures there's a single source of truth.
// ColumnType(<accepted data>) => [KingbaseType(s)...]
create_pg_mapping! {
  Boolean(Option<bool>) => [BOOL],
  Int32(Option<i32>) => [INT2, INT4, MYSQL_TINYINT, MYSQL_INT1, MYSQL_INT3, MYSQL_MEDIUMINT, MYSQL_MIDDLEINT, MYSQL_YEAR],
  Int64(Option<i64>) => [INT8, OID, MYSQL_UINT4, MYSQL_UINT8],
  Float(Option<f32>) => [FLOAT4],
  Double(Option<f64>) => [FLOAT8],
  Bytes(Option<Cow<'a, [u8]>>) => [BYTEA, MYSQL_BINARY, MYSQL_VARBINARY, MYSQL_SYS_BIT, MYSQL_BLOB, MYSQL_LONGBLOB, MYSQL_MEDIUMBLOB, MYSQL_TINYBLOB],
  Numeric(Option<bigdecimal::BigDecimal>) => [NUMERIC, MONEY],
  DateTime(Option<chrono::DateTime<chrono::Utc>>) => [TIMESTAMP, TIMESTAMPTZ, MYSQL_DATETIME, MYSQL_SYS_TIMESTAMP],
  Date(Option<chrono::NaiveDate>) => [DATE, MYSQL_SYS_DATE],
  Time(Option<chrono::NaiveTime>) => [TIME, TIMETZ, MYSQL_SYS_TIME],
  Text(Option<Cow<'a, str>>) => [INET, CIDR, BIT, VARBIT, MYSQL_LONGTEXT, MYSQL_MEDIUMTEXT, MYSQL_TINYTEXT, MYSQL_BPCHARBYTE, MYSQL_VARCHARBYTE],
  Uuid(Option<uuid::Uuid>) => [UUID],
  Json(Option<serde_json::Value>) => [JSON, JSONB, MYSQL_SYS_JSON],
  Xml(Option<Cow<'a, str>>) => [XML],
  Char(Option<char>) => [CHAR],

  // For the cases where the Postgres type is not directly mappable to ColumnType, use the following:
  // [KingbaseColumnType => ColumnType]
  [Enum => Text],
  [Set => Text],
  [Unknown => Text]
}

#[cfg(test)]
mod tests {
    use super::{ColumnType, KingbaseColumnType, ensure_supported_type, mysql_bit_length_from_type_modifier};
    use kingbase_tokio_postgres::types::{Kind, Type};

    #[test]
    fn sql_arrays_and_domains_over_arrays_are_unsupported() {
        let enum_type = Type::new("example".into(), 100_001, Kind::Enum(vec!["A".into()]), "public".into());
        let custom_array = Type::new("_example".into(), 100_002, Kind::Array(enum_type), "public".into());
        let domain_array = Type::new(
            "array_domain".into(),
            100_003,
            Kind::Domain(Type::INT4_ARRAY),
            "public".into(),
        );
        for typ in [
            Type::INT4_ARRAY,
            Type::TEXT_ARRAY,
            Type::JSONB_ARRAY,
            custom_array,
            domain_array,
        ] {
            assert!(ensure_supported_type(&typ).is_err(), "{typ}");
        }
        for typ in [
            Type::INT4,
            Type::TEXT,
            Type::JSONB,
            Type::MYSQL_SYS_JSON,
            Type::MYSQL_LONGBLOB,
        ] {
            assert!(ensure_supported_type(&typ).is_ok(), "{typ}");
        }
    }

    #[test]
    fn mysql_bit_typmod_is_the_declared_bit_width() {
        assert_eq!(mysql_bit_length_from_type_modifier(1), Some(1));
        assert_eq!(mysql_bit_length_from_type_modifier(8), Some(8));
        assert_eq!(mysql_bit_length_from_type_modifier(-1), None);
    }

    #[test]
    fn mysql_unsigned_types_are_int64() {
        assert_eq!(ColumnType::from(&Type::MYSQL_UINT4), ColumnType::Int64);
        assert_eq!(ColumnType::from(&Type::MYSQL_UINT8), ColumnType::Int64);
    }

    #[test]
    fn mysql_int1_is_int32() {
        assert_eq!(ColumnType::from(&Type::MYSQL_INT1), ColumnType::Int32);
    }

    #[test]
    fn mysql_set_is_text_and_mysql_enum_is_enum() {
        let set_type = Type::new("set_type".to_owned(), 100_001, Kind::MySqlSet, "sys".to_owned());
        let enum_type = Type::new(
            "enum_type".to_owned(),
            100_002,
            Kind::MySqlEnum(vec!["draft".to_owned(), "published".to_owned()]),
            "sys".to_owned(),
        );

        assert_eq!(ColumnType::from(&set_type), ColumnType::Text);
        assert_eq!(ColumnType::from(&enum_type), ColumnType::Enum);
        assert!(matches!(
            KingbaseColumnType::from_pg_type(&set_type),
            KingbaseColumnType::Set(_)
        ));
        assert!(matches!(
            KingbaseColumnType::from_pg_type(&enum_type),
            KingbaseColumnType::Enum(_)
        ));
    }
}
