use barrel::{types, Migration};
use pretty_assertions::assert_eq;
use sql_schema_describer::*;

mod common;
mod postgres;

use crate::common::*;
use crate::postgres::*;

#[tokio::test]
async fn all_postgres_column_types_must_work() {
    let mut migration = Migration::new().schema(SCHEMA);
    migration.create_table("User", move |t| {
        t.add_column("array_bin_col", types::array(&types::binary()));
        t.add_column("array_bool_col", types::array(&types::boolean()));
        t.add_column("array_date_col", types::array(&types::date()));
        t.add_column("array_double_col", types::array(&types::double()));
        t.add_column("array_float_col", types::array(&types::float()));
        t.add_column("array_int_col", types::array(&types::integer()));
        t.add_column("array_text_col", types::array(&types::text()));
        t.add_column("array_varchar_col", types::array(&types::varchar(255)));
        t.add_column("bigint_col", types::custom("BIGINT"));
        t.add_column("bigserial_col", types::custom("BIGSERIAL"));
        t.add_column("bit_col", types::custom("BIT"));
        t.add_column("bit_varying_col", types::custom("BIT VARYING(1)"));
        t.add_column("binary_col", types::binary());
        t.add_column("boolean_col", types::boolean());
        t.add_column("box_col", types::custom("BOX"));
        t.add_column("char_col", types::custom("CHARACTER(1)"));
        t.add_column("circle_col", types::custom("CIRCLE"));
        t.add_column("date_time_col", types::date());
        t.add_column("double_col", types::double());
        t.add_column("float_col", types::float());
        t.add_column("int_col", types::integer());
        t.add_column("interval_col", types::custom("INTERVAL"));
        t.add_column("line_col", types::custom("LINE"));
        t.add_column("lseg_col", types::custom("LSEG"));
        t.add_column("numeric_col", types::custom("NUMERIC"));
        t.add_column("path_col", types::custom("PATH"));
        t.add_column("pg_lsn_col", types::custom("PG_LSN"));
        t.add_column("polygon_col", types::custom("POLYGON"));
        t.add_column("smallint_col", types::custom("SMALLINT"));
        t.add_column("smallserial_col", types::custom("SMALLSERIAL"));
        t.add_column("serial_col", types::custom("SERIAL"));
        // TODO: Test also autoincrement variety
        t.add_column("primary_col", types::primary());
        t.add_column("string1_col", types::text());
        t.add_column("string2_col", types::varchar(1));
        t.add_column("time_col", types::custom("TIME"));
        t.add_column("time_with_zone_col", types::custom("TIME WITH TIME ZONE"));
        t.add_column("timestamp_col", types::custom("TIMESTAMP"));
        t.add_column("timestamp_with_zone_col", types::custom("TIMESTAMP WITH TIME ZONE"));
        t.add_column("tsquery_col", types::custom("TSQUERY"));
        t.add_column("tsvector_col", types::custom("TSVECTOR"));
        t.add_column("txid_col", types::custom("TXID_SNAPSHOT"));
        t.add_column("json_col", types::json());
        t.add_column("jsonb_col", types::custom("JSONB"));
        t.add_column("uuid_col", types::uuid());
    });

    let full_sql = migration.make::<barrel::backend::Pg>();
    let inspector = get_postgres_describer(&full_sql, "all_postgres_column_types_must_work").await;
    let result = inspector.describe(SCHEMA).await.expect("describing");
    let mut table = result.get_table("User").expect("couldn't get User table").to_owned();
    // Ensure columns are sorted as expected when comparing
    table.columns.sort_unstable_by_key(|c| c.name.to_owned());
    let mut expected_columns = vec![
        Column {
            name: "array_bin_col".into(),
            tpe: ColumnType {
                raw: "_bytea".into(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::List,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_bool_col".into(),
            tpe: ColumnType {
                raw: "_bool".into(),
                family: ColumnTypeFamily::Boolean,
                arity: ColumnArity::List,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_date_col".into(),
            tpe: ColumnType {
                raw: "_date".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::List,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_double_col".into(),
            tpe: ColumnType {
                raw: "_float8".into(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::List,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_float_col".into(),
            tpe: ColumnType {
                raw: "_float8".into(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::List,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_int_col".into(),
            tpe: ColumnType {
                raw: "_int4".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::List,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_text_col".into(),
            tpe: ColumnType {
                raw: "_text".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::List,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_varchar_col".into(),
            tpe: ColumnType {
                raw: "_varchar".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::List,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "binary_col".into(),
            tpe: ColumnType {
                raw: "bytea".into(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "boolean_col".into(),
            tpe: ColumnType {
                raw: "bool".into(),
                family: ColumnTypeFamily::Boolean,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "date_time_col".into(),
            tpe: ColumnType {
                raw: "date".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "double_col".into(),
            tpe: ColumnType {
                raw: "float8".into(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "float_col".into(),
            tpe: ColumnType {
                raw: "float8".into(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "int_col".into(),
            tpe: ColumnType {
                raw: "int4".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "primary_col".into(),
            tpe: ColumnType {
                raw: "int4".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: Some(format!("nextval(\"{}\".\"User_primary_col_seq\"::regclass)", SCHEMA)),
            auto_increment: true,
        },
        Column {
            name: "string1_col".into(),
            tpe: ColumnType {
                raw: "text".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "string2_col".into(),
            tpe: ColumnType {
                raw: "varchar".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "bigint_col".into(),
            tpe: ColumnType {
                raw: "int8".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "bigserial_col".into(),
            tpe: ColumnType {
                raw: "int8".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: Some(format!("nextval(\"{}\".\"User_bigserial_col_seq\"::regclass)", SCHEMA)),
            auto_increment: true,
        },
        Column {
            name: "bit_col".into(),
            tpe: ColumnType {
                raw: "bit".into(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "bit_varying_col".into(),
            tpe: ColumnType {
                raw: "varbit".into(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "box_col".into(),
            tpe: ColumnType {
                raw: "box".into(),
                family: ColumnTypeFamily::Geometric,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "char_col".into(),
            tpe: ColumnType {
                raw: "bpchar".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "circle_col".into(),
            tpe: ColumnType {
                raw: "circle".into(),
                family: ColumnTypeFamily::Geometric,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "interval_col".into(),
            tpe: ColumnType {
                raw: "interval".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "line_col".into(),
            tpe: ColumnType {
                raw: "line".into(),
                family: ColumnTypeFamily::Geometric,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "lseg_col".into(),
            tpe: ColumnType {
                raw: "lseg".into(),
                family: ColumnTypeFamily::Geometric,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "numeric_col".into(),
            tpe: ColumnType {
                raw: "numeric".into(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "path_col".into(),
            tpe: ColumnType {
                raw: "path".into(),
                family: ColumnTypeFamily::Geometric,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "pg_lsn_col".into(),
            tpe: ColumnType {
                raw: "pg_lsn".into(),
                family: ColumnTypeFamily::LogSequenceNumber,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "polygon_col".into(),
            tpe: ColumnType {
                raw: "polygon".into(),
                family: ColumnTypeFamily::Geometric,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallint_col".into(),
            tpe: ColumnType {
                raw: "int2".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallserial_col".into(),
            tpe: ColumnType {
                raw: "int2".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: Some(format!(
                "nextval(\"{}\".\"User_smallserial_col_seq\"::regclass)",
                SCHEMA
            )),
            auto_increment: true,
        },
        Column {
            name: "serial_col".into(),
            tpe: ColumnType {
                raw: "int4".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: Some(format!("nextval(\"{}\".\"User_serial_col_seq\"::regclass)", SCHEMA)),
            auto_increment: true,
        },
        Column {
            name: "time_col".into(),
            tpe: ColumnType {
                raw: "time".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "time_with_zone_col".into(),
            tpe: ColumnType {
                raw: "timetz".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "timestamp_col".into(),
            tpe: ColumnType {
                raw: "timestamp".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "timestamp_with_zone_col".into(),
            tpe: ColumnType {
                raw: "timestamptz".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tsquery_col".into(),
            tpe: ColumnType {
                raw: "tsquery".into(),
                family: ColumnTypeFamily::TextSearch,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tsvector_col".into(),
            tpe: ColumnType {
                raw: "tsvector".into(),
                family: ColumnTypeFamily::TextSearch,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "txid_col".into(),
            tpe: ColumnType {
                raw: "txid_snapshot".into(),
                family: ColumnTypeFamily::TransactionId,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "json_col".into(),
            tpe: ColumnType {
                raw: "json".into(),
                family: ColumnTypeFamily::Json,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "jsonb_col".into(),
            tpe: ColumnType {
                raw: "jsonb".into(),
                family: ColumnTypeFamily::Json,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "uuid_col".into(),
            tpe: ColumnType {
                raw: "uuid".into(),
                family: ColumnTypeFamily::Uuid,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
    ];
    expected_columns.sort_unstable_by_key(|c| c.name.to_owned());

    assert_eq!(
        table,
        Table {
            name: "User".into(),
            columns: expected_columns,
            indices: vec![Index {
                name: "User_uuid_col_key".into(),
                columns: vec!["uuid_col".into(),],
                tpe: IndexType::Unique,
            },],
            primary_key: Some(PrimaryKey {
                columns: vec!["primary_col".into()],
                sequence: Some(Sequence {
                    name: "User_primary_col_seq".into(),
                    initial_value: 1,
                    allocation_size: 1,
                },),
            }),
            foreign_keys: vec![],
        }
    );
}

#[tokio::test]
async fn postgres_foreign_key_on_delete_must_be_handled() {
    let sql = format!(
        "CREATE TABLE \"{0}\".\"City\" (id INT PRIMARY KEY);
         CREATE TABLE \"{0}\".\"User\" (
            id INT PRIMARY KEY,
            city INT REFERENCES \"{0}\".\"City\" (id) ON DELETE NO ACTION,
            city_cascade INT REFERENCES \"{0}\".\"City\" (id) ON DELETE CASCADE,
            city_restrict INT REFERENCES \"{0}\".\"City\" (id) ON DELETE RESTRICT,
            city_set_null INT REFERENCES \"{0}\".\"City\" (id) ON DELETE SET NULL,
            city_set_default INT REFERENCES \"{0}\".\"City\" (id) ON DELETE SET DEFAULT
        );
        ",
        SCHEMA
    );
    let inspector = get_postgres_describer(&sql, "postgres_foreign_key_on_delete_must_be_handled").await;

    let schema = inspector.describe(SCHEMA).await.expect("describing");
    let mut table = schema.get_table("User").expect("get User table").to_owned();
    table.foreign_keys.sort_unstable_by_key(|fk| fk.columns.clone());

    assert_eq!(
        table,
        Table {
            name: "User".into(),
            columns: vec![
                Column {
                    name: "city".into(),
                    tpe: ColumnType {
                        raw: "int4".into(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_cascade".into(),
                    tpe: ColumnType {
                        raw: "int4".into(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_restrict".into(),
                    tpe: ColumnType {
                        raw: "int4".into(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_set_default".into(),
                    tpe: ColumnType {
                        raw: "int4".into(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_set_null".into(),
                    tpe: ColumnType {
                        raw: "int4".into(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "id".into(),
                    tpe: ColumnType {
                        raw: "int4".into(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Required,
                    },

                    default: None,
                    auto_increment: false,
                },
            ],
            indices: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["id".into()],
                sequence: None,
            }),
            foreign_keys: vec![
                ForeignKey {
                    constraint_name: Some("User_city_fkey".to_owned()),
                    columns: vec!["city".into()],
                    referenced_columns: vec!["id".into()],
                    referenced_table: "City".into(),
                    on_delete_action: ForeignKeyAction::NoAction,
                },
                ForeignKey {
                    constraint_name: Some("User_city_cascade_fkey".to_owned()),
                    columns: vec!["city_cascade".into()],
                    referenced_columns: vec!["id".into()],
                    referenced_table: "City".into(),
                    on_delete_action: ForeignKeyAction::Cascade,
                },
                ForeignKey {
                    constraint_name: Some("User_city_restrict_fkey".to_owned()),
                    columns: vec!["city_restrict".into()],
                    referenced_columns: vec!["id".into()],
                    referenced_table: "City".into(),
                    on_delete_action: ForeignKeyAction::Restrict,
                },
                ForeignKey {
                    constraint_name: Some("User_city_set_default_fkey".to_owned()),
                    columns: vec!["city_set_default".into()],
                    referenced_columns: vec!["id".into()],
                    referenced_table: "City".into(),
                    on_delete_action: ForeignKeyAction::SetDefault,
                },
                ForeignKey {
                    constraint_name: Some("User_city_set_null_fkey".to_owned()),
                    columns: vec!["city_set_null".into()],
                    referenced_columns: vec!["id".into()],
                    referenced_table: "City".into(),
                    on_delete_action: ForeignKeyAction::SetNull,
                },
            ],
        }
    );
}

#[tokio::test]
async fn postgres_enums_must_work() {
    let inspector = get_postgres_describer(
        &format!("CREATE TYPE \"{}\".\"mood\" AS ENUM ('sad', 'ok', 'happy')", SCHEMA),
        "postgres_enums_must_work",
    )
    .await;

    let schema = inspector.describe(SCHEMA).await.expect("describing");
    let got_enum = schema.get_enum("mood").expect("get enum");

    let values: Vec<String> = vec!["sad".into(), "ok".into(), "happy".into()];
    assert_eq!(
        got_enum,
        &Enum {
            name: "mood".into(),
            values,
        }
    );
}

#[tokio::test]
async fn postgres_sequences_must_work() {
    let inspector = get_postgres_describer(
        &format!("CREATE SEQUENCE \"{}\".\"test\"", SCHEMA),
        "postgres_sequences_must_work",
    )
    .await;

    let schema = inspector.describe(SCHEMA).await.expect("describing");
    let got_seq = schema.get_sequence("test").expect("get sequence");

    assert_eq!(
        got_seq,
        &Sequence {
            name: "test".into(),
            initial_value: 1,
            allocation_size: 1,
        },
    );
}
