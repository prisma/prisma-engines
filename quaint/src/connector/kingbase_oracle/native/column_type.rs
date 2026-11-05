use crate::{ast::Value, connector::ColumnType};
use kingbase_tokio_postgres::types::{Kind, Type};
use std::borrow::Cow;

macro_rules! create_pg_mapping {
    (
        $($key:ident($typ:ty) => [$($value:ident),+]),* $(,)?
        $([$metadata_key:ident => [$($metadata_value:ident),+]]),* $(,)?
    ) => {
        $(
            concat_idents::concat_idents!(enum_name = KingbaseColumnType, $key {
                #[derive(Debug)]
                #[allow(non_camel_case_types)]
                #[allow(clippy::upper_case_acronyms)]
                pub(crate) enum enum_name {
                    $($value,)*
                }
            });

            concat_idents::concat_idents!(struct_name = KingbaseColumnValidator, $key {
                #[derive(Debug)]
                pub(crate) struct struct_name;

                impl struct_name {
                    #[inline]
                    #[allow(clippy::extra_unused_lifetimes)]
                    pub(crate) fn read<'a>(&self, val: $typ) -> $typ {
                        val
                    }
                }
            });
        )*

        pub(crate) enum KingbaseColumnType {
            $(
                $key(
                    concat_idents::concat_idents!(enum_name = KingbaseColumnType, $key, { enum_name }),
                    concat_idents::concat_idents!(struct_name = KingbaseColumnValidator, $key, { struct_name }),
                ),
            )*
            Enum(KingbaseColumnValidatorText),
            // Some types have query metadata but no native binary decoder.
            Unsupported(ColumnType),
        }

        impl KingbaseColumnType {
            #[deny(unreachable_patterns)]
            pub(crate) fn from_pg_type(typ: &Type) -> Self {
                match typ {
                    $(
                        $(
                            &Type::$value => Self::$key(
                                concat_idents::concat_idents!(enum_name = KingbaseColumnType, $key, { enum_name::$value }),
                                concat_idents::concat_idents!(struct_name = KingbaseColumnValidator, $key, { struct_name }),
                            ),
                        )*
                    )*
                    $(
                        $(&Type::$metadata_value => Self::Unsupported(ColumnType::$metadata_key),)*
                    )*
                    typ => match typ.kind() {
                        Kind::Enum(_) => Self::Enum(KingbaseColumnValidatorText),
                        _ => Self::Unsupported(ColumnType::Unknown),
                    },
                }
            }
        }

        impl From<KingbaseColumnType> for ColumnType {
            fn from(typ: KingbaseColumnType) -> Self {
                match typ {
                    $(KingbaseColumnType::$key(..) => Self::$key,)*
                    KingbaseColumnType::Enum(_) => Self::Enum,
                    KingbaseColumnType::Unsupported(column_type) => column_type,
                }
            }
        }
    };
}

// The mapping drives both result metadata and the exhaustive decoding branches.
create_pg_mapping! {
    Boolean(Option<bool>) => [BOOL],
    Int32(Option<i32>) => [INT2, INT4],
    Int64(Option<i64>) => [INT8, OID],
    Float(Option<f32>) => [FLOAT4],
    Double(Option<f64>) => [FLOAT8],
    Numeric(Option<bigdecimal::BigDecimal>) => [NUMERIC],
    Text(Option<Cow<'a, str>>) => [TEXT, VARCHAR, BPCHAR, NAME, UNKNOWN, ORACLE_UROWID, ORACLE_CLOB, ORACLE_NCLOB, ORACLE_BPCHARBYTE, ORACLE_VARCHARBYTE, ORACLE_BFILE],
    Bytes(Option<Cow<'a, [u8]>>) => [BYTEA, ORACLE_BLOB],
    Json(Option<serde_json::Value>) => [JSON, JSONB],
    Xml(Option<Cow<'a, str>>) => [XML],
    Uuid(Option<uuid::Uuid>) => [UUID],
    Char(Option<char>) => [CHAR],
    DateTime(Option<chrono::DateTime<chrono::Utc>>) => [TIMESTAMP, TIMESTAMPTZ, ORACLE_SYS_DATE],
    Date(Option<chrono::NaiveDate>) => [DATE],
    Time(Option<chrono::NaiveTime>) => [TIME],

    // Catalog decoding only; this does not enable array parameters or scalar lists.
    TextArray(Option<Vec<Value<'static>>>) => [TEXT_ARRAY, NAME_ARRAY, VARCHAR_ARRAY],

    // Text-protocol query metadata does not imply native binary codec support.
    [Int32 => [ORACLE_TINYINT]],
    [Text => [ORACLE_ROWID]],
    [Time => [TIMETZ]],
}

/// Maps Kingbase Oracle-mode PostgreSQL-wire OIDs to Quaint result types.
pub(crate) fn column_type_from_type(typ: &Type) -> ColumnType {
    // Both Kingbase modes share Type, so a second From<&Type> impl would overlap.
    KingbaseColumnType::from_pg_type(typ).into()
}

#[cfg(test)]
mod tests {
    use super::{KingbaseColumnType, column_type_from_type};
    use crate::connector::ColumnType;
    use kingbase_tokio_postgres::types::Type;

    #[tokio::test]
    async fn describe_rowid_reports_text_for_typed_sql() {
        use crate::{prelude::Queryable, single::Quaint, tests::test_api::kingbase_oracle::CONN_STR};
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        let described = connection
            .describe_query("SELECT nextrowid() AS \"rowId\", CAST(-128 AS TINYINT) AS \"tiny\"")
            .await
            .unwrap();
        assert_eq!(described.columns.len(), 2);
        assert_eq!(described.columns[0].typ, ColumnType::Text);
        assert_eq!(described.columns[0].name, "rowId");
        assert_eq!(described.columns[1].typ, ColumnType::Int32);
    }

    #[tokio::test]
    async fn integer_decoding_matches_metadata_even_without_rows() {
        use crate::{Value, prelude::Queryable, single::Quaint, tests::test_api::kingbase_oracle::CONN_STR};

        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        let sql = "SELECT CAST(-32768 AS SMALLINT) AS \"small\", CAST(42 AS INTEGER) AS \"int\", \
                   CAST(9223372036854775807 AS BIGINT) AS \"big\", CAST(NULL AS SMALLINT) AS \"null\"";
        let expected_types = vec![
            ColumnType::Int32,
            ColumnType::Int32,
            ColumnType::Int64,
            ColumnType::Int32,
        ];

        let result = connection.query_raw(sql, &[]).await.unwrap();
        assert_eq!(result.types(), expected_types);
        let row = result.into_single().unwrap();
        assert_eq!(row.at(0), Some(&Value::int32(-32768)));
        assert_eq!(row.at(1), Some(&Value::int32(42)));
        assert_eq!(row.at(2), Some(&Value::int64(i64::MAX)));
        assert_eq!(row.at(3), Some(&Value::null_int32()));

        let empty = connection.query_raw(&format!("{sql} WHERE 1 = 0"), &[]).await.unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.types(), expected_types);
    }

    #[test]
    fn maps_oracle_mode_scalar_oids() {
        assert_eq!(column_type_from_type(&Type::NUMERIC), ColumnType::Numeric);
        assert_eq!(column_type_from_type(&Type::ORACLE_TINYINT), ColumnType::Int32);
        assert_eq!(column_type_from_type(&Type::ORACLE_BLOB), ColumnType::Bytes);
        assert_eq!(column_type_from_type(&Type::ORACLE_CLOB), ColumnType::Text);
        assert_eq!(column_type_from_type(&Type::ORACLE_NCLOB), ColumnType::Text);
        assert_eq!(column_type_from_type(&Type::ORACLE_SYS_DATE), ColumnType::DateTime);
        assert_eq!(column_type_from_type(&Type::ORACLE_ROWID), ColumnType::Text);
        assert_eq!(column_type_from_type(&Type::TEXT_ARRAY), ColumnType::TextArray);
    }

    #[test]
    fn recognises_oracle_character_types() {
        assert_eq!(column_type_from_type(&Type::ORACLE_UROWID), ColumnType::Text);
        assert_eq!(column_type_from_type(&Type::ORACLE_BPCHARBYTE), ColumnType::Text);
        assert_eq!(column_type_from_type(&Type::ORACLE_VARCHARBYTE), ColumnType::Text);
        assert_eq!(column_type_from_type(&Type::ORACLE_BLOB), ColumnType::Bytes);
        assert_eq!(column_type_from_type(&Type::TEXT_ARRAY), ColumnType::TextArray);
        assert_eq!(column_type_from_type(&Type::BOOL_ARRAY), ColumnType::Unknown);
    }

    #[test]
    fn metadata_only_types_do_not_enable_binary_decoding() {
        for (typ, expected) in [
            (Type::ORACLE_TINYINT, ColumnType::Int32),
            (Type::ORACLE_ROWID, ColumnType::Text),
            (Type::TIMETZ, ColumnType::Time),
            (Type::BOOL_ARRAY, ColumnType::Unknown),
        ] {
            assert_eq!(column_type_from_type(&typ), expected);
            assert!(matches!(
                KingbaseColumnType::from_pg_type(&typ),
                KingbaseColumnType::Unsupported(column_type) if column_type == expected
            ));
        }
    }
}
