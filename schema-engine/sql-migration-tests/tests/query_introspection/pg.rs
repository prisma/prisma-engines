use super::utils::*;

use psl::builtin_connectors::{CockroachType, PostgresType};
use quaint::prelude::ColumnType;
use sql_migration_tests::test_api::*;

mod common {
    use super::*;

    #[test_connector(tags(Postgres))]
    fn empty_result(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT int FROM model WHERE 1 = 0 AND int = $1;",
                documentation: "",
                parameters: [
                    IntrospectSqlQueryParameterOutput {
                        documentation: "",
                        name: "int4",
                        typ: "int",
                    },
                ],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "int",
                        typ: "int",
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT int FROM model WHERE 1 = 0 AND int = ?;")
            .send_sync()
            .expect_result(expected)
    }

    #[test_connector(tags(Postgres, CockroachDb))]
    fn custom_enum(api: TestApi) {
        api.schema_push(ENUM_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "INSERT INTO model (id, enum) VALUES ($1, $2) RETURNING id, enum;",
                documentation: "",
                parameters: [
                    IntrospectSqlQueryParameterOutput {
                        documentation: "",
                        name: "int4",
                        typ: "int",
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: "",
                        name: "MyFancyEnum",
                        typ: "MyFancyEnum",
                    },
                ],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "enum",
                        typ: "MyFancyEnum",
                    },
                ],
            }
        "#]];

        api.introspect_sql(
            "test_1",
            "INSERT INTO model (id, enum) VALUES (?, ?) RETURNING id, enum;",
        )
        .send_sync()
        .expect_result(expected)
    }
}

mod pg {
    use super::*;

    const PG_DATASOURCE: &str = r#"
        datasource db {
            provider = "postgres"
            url      = "postgresql://localhost:5432"
        }
    "#;

    #[test_connector(tags(Postgres), exclude(CockroachDb))]
    fn named_expr(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT 1 + 1 as add;",
                documentation: "",
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "add",
                        typ: "int",
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT 1 + 1 as add;")
            .send_sync()
            .expect_result(expected)
    }

    #[test_connector(tags(Postgres), exclude(CockroachDb))]
    fn unnamed_expr(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT 1 + 1;",
                documentation: "",
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "?column?",
                        typ: "int",
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT 1 + 1;")
            .send_sync()
            .expect_result(expected)
    }

    macro_rules! test_native_types_pg {
        (
            $($test_name:ident($nt:expr) => $ct:ident,)*
        ) => {
            $(
                paste::paste! {
                    #[test_connector(tags(Postgres), exclude(CockroachDb))]
                    fn $test_name(api: TestApi) {
                        let dm = render_native_type_datamodel::<PostgresType>(&api, PG_DATASOURCE, $nt.to_parts(), $nt);

                        if PostgresType::Citext == $nt {
                            api.raw_cmd("CREATE EXTENSION IF NOT EXISTS citext;");
                        }

                        api.schema_push(&dm).send();

                        let query = "INSERT INTO test (field) VALUES (?) RETURNING field;";

                        api.introspect_sql("test", query)
                            .send_sync()
                            .expect_param_type(0, ColumnType::$ct)
                            .expect_column_type(0, ColumnType::$ct);
                    }
                }
            )*
        };
    }

    test_native_types_pg! {
        small_int(PostgresType::SmallInt) => Int32,
        integer(PostgresType::Integer) => Int32,
        big_int(PostgresType::BigInt) => Int64,
        nt_decimal(PostgresType::Decimal(Some((4, 4)))) => Numeric,
        money(PostgresType::Money) => Numeric,
        inet(PostgresType::Inet) => Text,
        oid(PostgresType::Oid) => Int64,
        citext(PostgresType::Citext) => Text,
        real(PostgresType::Real) => Float,
        double(PostgresType::DoublePrecision) => Double,
        var_char(PostgresType::VarChar(Some(255))) => Text,
        char(PostgresType::Char(Some(255))) => Text,
        text(PostgresType::Text) => Text,
        byte(PostgresType::ByteA) => Bytes,
        timestamp(PostgresType::Timestamp(Some(1))) => DateTime,
        timestamptz(PostgresType::Timestamptz(Some(1))) => DateTime,
        date(PostgresType::Date) => Date,
        time(PostgresType::Time(Some(1))) => Time,
        timetz(PostgresType::Timetz(Some(1))) => Time,
        boolean(PostgresType::Boolean) => Boolean,
        bit(PostgresType::Bit(Some(1))) => Text,
        var_bit(PostgresType::VarBit(Some(1))) => Text,
        uuid(PostgresType::Uuid) => Uuid,
        xml(PostgresType::Xml) => Xml,
        nt_json(PostgresType::Json) => Json,
        json_b(PostgresType::JsonB) => Json,
    }
}

mod crdb {
    use super::*;

    #[test_connector(tags(CockroachDb))]
    fn unnamed_expr_crdb(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT 1 + 1;",
                documentation: "",
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "?column?",
                        typ: "bigint",
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT 1 + 1;")
            .send_sync()
            .expect_result(expected)
    }

    #[test_connector(tags(CockroachDb))]
    fn named_expr_crdb(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT 1 + 1 as add;",
                documentation: "",
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "add",
                        typ: "bigint",
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT 1 + 1 as add;")
            .send_sync()
            .expect_result(expected)
    }

    macro_rules! test_native_types_crdb {
    (
        $($test_name:ident($nt:expr) => $ct:ident,)*
    ) => {
        $(
            paste::paste! {
                #[test_connector(tags(CockroachDb))]
                fn [<crdb _ $test_name>](api: TestApi) {
                    let dm = render_native_type_datamodel::<CockroachType>(&api, CRDB_DATASOURCE, $nt.to_parts(), $nt);

                    api.schema_push(&dm).send();

                    let query = "INSERT INTO test (id, field) VALUES (?, ?) RETURNING field;";

                    api.introspect_sql("test", query)
                        .send_sync()
                        .expect_param_type(1, ColumnType::$ct)
                        .expect_column_type(0, ColumnType::$ct);
                }
            }
        )*
    };
}

    const CRDB_DATASOURCE: &str = r#"
  datasource db {
    provider = "cockroachdb"
    url      = "postgresql://localhost:5432"
}
"#;

    test_native_types_crdb! {
        bit(CockroachType::Bit(Some(1))) => Text,
        boolean(CockroachType::Bool) => Boolean,
        nt_bytes(CockroachType::Bytes) => Bytes,
        char(CockroachType::Char(Some(255))) => Text,
        date(CockroachType::Date) => Date,
        nt_decimal(CockroachType::Decimal(Some((4, 4)))) => Numeric,
        float4(CockroachType::Float4) => Float,
        float8(CockroachType::Float8) => Double,
        inet(CockroachType::Inet) => Text,
        int2(CockroachType::Int2) => Int32,
        int4(CockroachType::Int4) => Int32,
        int8(CockroachType::Int8) => Int64,
        json_b(CockroachType::JsonB) => Json,
        oid(CockroachType::Oid) => Int64,
        catalog_single_char(CockroachType::CatalogSingleChar) => Char,
        nt_string(CockroachType::String(Some(255))) => Text,
        time(CockroachType::Time(Some(1))) => Time,
        timestamp(CockroachType::Timestamp(Some(1))) => DateTime,
        timestamptz(CockroachType::Timestamptz(Some(1))) => DateTime,
        timetz(CockroachType::Timetz(Some(1))) => Time,
        uuid(CockroachType::Uuid) => Uuid,
        var_bit(CockroachType::VarBit(Some(1))) => Text,
    }
}
