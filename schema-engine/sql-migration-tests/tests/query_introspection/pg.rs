use super::utils::*;

use psl::builtin_connectors::{CockroachType, KnownPostgresType, PostgresType};
use quaint::prelude::ColumnType;
use sql_migration_tests::test_api::*;

mod common {
    use super::*;

    #[test_connector(tags(Postgres))]
    fn insert(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let query = "INSERT INTO model (int, string, bigint, float, bytes, bool, dt) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING int, string, bigint, float, bytes, bool, dt;";
        let res = api.introspect_sql("test_1", query).send_sync();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "INSERT INTO model (int, string, bigint, float, bytes, bool, dt) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING int, string, bigint, float, bytes, bool, dt;",
                documentation: None,
                parameters: [
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "int4",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "text",
                        typ: "string",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "int8",
                        typ: "bigint",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "float8",
                        typ: "double",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "bytea",
                        typ: "bytes",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "bool",
                        typ: "bool",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "timestamp",
                        typ: "datetime",
                        nullable: false,
                    },
                ],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "int",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "string",
                        typ: "string",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "bigint",
                        typ: "bigint",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "float",
                        typ: "double",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "bytes",
                        typ: "bytes",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "bool",
                        typ: "bool",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "dt",
                        typ: "datetime",
                        nullable: false,
                    },
                ],
            }
        "#]];

        res.expect_result(expected);
    }

    #[test_connector(tags(Postgres))]
    fn insert_nullable(api: TestApi) {
        api.schema_push(SIMPLE_NULLABLE_SCHEMA).send().assert_green();

        let query = "INSERT INTO model (int, string, bigint, float, bytes, bool, dt) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING int, string, bigint, float, bytes, bool, dt;";
        let res = api.introspect_sql("test_1", query).send_sync();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "INSERT INTO model (int, string, bigint, float, bytes, bool, dt) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING int, string, bigint, float, bytes, bool, dt;",
                documentation: None,
                parameters: [
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "int4",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "text",
                        typ: "string",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "int8",
                        typ: "bigint",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "float8",
                        typ: "double",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "bytea",
                        typ: "bytes",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "bool",
                        typ: "bool",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "timestamp",
                        typ: "datetime",
                        nullable: false,
                    },
                ],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "int",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "string",
                        typ: "string",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "bigint",
                        typ: "bigint",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "float",
                        typ: "double",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "bytes",
                        typ: "bytes",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "bool",
                        typ: "bool",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "dt",
                        typ: "datetime",
                        nullable: true,
                    },
                ],
            }
        "#]];

        res.expect_result(expected);
    }

    #[test_connector(tags(Postgres))]
    fn empty_result(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT int FROM model WHERE 1 = 0 AND int = $1;",
                documentation: None,
                parameters: [
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "int4",
                        typ: "int",
                        nullable: false,
                    },
                ],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "int",
                        typ: "int",
                        nullable: false,
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
                documentation: None,
                parameters: [
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "int4",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryParameterOutput {
                        documentation: None,
                        name: "MyFancyEnum",
                        typ: "string",
                        nullable: false,
                    },
                ],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "enum",
                        typ: "MyFancyEnum",
                        nullable: false,
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

mod postgres {
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
                source: "SELECT 1 + 1 as \"add\";",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "add",
                        typ: "int",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT 1 + 1 as \"add\";")
            .send_sync()
            .expect_result(expected)
    }

    #[test_connector(tags(Postgres), exclude(CockroachDb))]
    fn mixed_named_expr(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT \"int\" + 1 as \"add\" FROM \"model\";",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "add",
                        typ: "int",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT \"int\" + 1 as \"add\" FROM \"model\";")
            .send_sync()
            .expect_result(expected)
    }

    #[test_connector(tags(Postgres), exclude(CockroachDb))]
    fn mixed_unnamed_expr(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            ConnectorErrorImpl {
                user_facing_error: None,
                message: Some(
                    "Invalid input provided to query: Invalid column name '?column?' for index 0. Your SQL query must explicitly alias that column name.",
                ),
                source: None,
                context: SpanTrace [],
            }
            Invalid input provided to query: Invalid column name '?column?' for index 0. Your SQL query must explicitly alias that column name.

        "#]];

        expected.assert_debug_eq(
            &api.introspect_sql("test_1", "SELECT \"int\" + 1 FROM \"model\";")
                .send_unwrap_err(),
        );
    }

    #[test_connector(tags(Postgres), exclude(CockroachDb))]
    fn mixed_expr_cast(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT CAST(\"int\" + 1 as int) FROM model;",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "int4",
                        typ: "int",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT CAST(\"int\" + 1 as int) FROM model;")
            .send_sync()
            .expect_result(expected)
    }

    #[test_connector(tags(Postgres), exclude(CockroachDb))]
    fn subquery(api: TestApi) {
        api.schema_push(SIMPLE_NULLABLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT int, foo.int, foo.string FROM (SELECT * FROM model) AS foo",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "int",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "int",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "string",
                        typ: "string",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql(
            "test_1",
            "SELECT int, foo.int, foo.string FROM (SELECT * FROM model) AS foo",
        )
        .send_sync()
        .expect_result(expected)
    }

    #[test_connector(tags(Postgres), exclude(CockroachDb))]
    fn left_join(api: TestApi) {
        api.schema_push(RELATION_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT parent.id as parentId, parent.nullable as parentNullable, child.id as childId, child.nullable as childNullable FROM parent LEFT JOIN child ON parent.id = child.parent_id",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "parentid",
                        typ: "int",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "parentnullable",
                        typ: "string",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "childid",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "childnullable",
                        typ: "string",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT parent.id as parentId, parent.nullable as parentNullable, child.id as childId, child.nullable as childNullable FROM parent LEFT JOIN child ON parent.id = child.parent_id")
        .send_sync()
        .expect_result(expected)
    }

    // test nullability inference for various joins
    #[test_connector(tags(Postgres), exclude(CockroachDb))]
    fn outer_join(api: TestApi) {
        api.schema_push(
            "model products {
                    product_no Int     @id
                    name       String?
                }

                model tweet {
                    id   Int    @id @default(autoincrement())
                    text String
                }",
        )
        .send()
        .assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "select tweet.id from (values (null)) vals(val) inner join tweet on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                ],
            }
        "#]];

        // inner join, nullability should not be overridden
        api.introspect_sql(
            "test_1",
            "select tweet.id from (values (null)) vals(val) inner join tweet on false",
        )
        .send_sync()
        .expect_result(expected);

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_2",
                source: "select tweet.id from (values (null)) vals(val) left join tweet on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: true,
                    },
                ],
            }
        "#]];

        // tweet.id is marked NOT NULL but it's brought in from a left-join here
        // which should make it nullable
        api.introspect_sql(
            "test_2",
            "select tweet.id from (values (null)) vals(val) left join tweet on false",
        )
        .send_sync()
        .expect_result(expected);

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_3",
                source: "select tweet1.id, tweet2.id from tweet tweet1 left join tweet tweet2 on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: true,
                    },
                ],
            }
        "#]];

        // make sure we don't mis-infer for the outer half of the join
        api.introspect_sql(
            "test_3",
            "select tweet1.id, tweet2.id from tweet tweet1 left join tweet tweet2 on false",
        )
        .send_sync()
        .expect_result(expected);

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_4",
                source: "select tweet1.id, tweet2.id from tweet tweet1 right join tweet tweet2 on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                ],
            }
        "#]];

        // right join, nullability should be inverted
        api.introspect_sql(
            "test_4",
            "select tweet1.id, tweet2.id from tweet tweet1 right join tweet tweet2 on false",
        )
        .send_sync()
        .expect_result(expected);

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_5",
                source: "select tweet1.id, tweet2.id from tweet tweet1 full join tweet tweet2 on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: true,
                    },
                ],
            }
        "#]];

        // right join, nullability should be inverted
        api.introspect_sql(
            "test_5",
            "select tweet1.id, tweet2.id from tweet tweet1 full join tweet tweet2 on false",
        )
        .send_sync()
        .expect_result(expected);
    }

    macro_rules! test_native_types_pg {
        (
            $($test_name:ident($nt:expr) => $ct:ident,)*
        ) => {
            $(
                paste::paste! {
                    #[test_connector(tags(Postgres), exclude(CockroachDb))]
                    fn $test_name(api: TestApi) {
                        let dm = render_native_type_datamodel::<PostgresType>(&api, PG_DATASOURCE, $nt.to_parts(), PostgresType::Known($nt));

                        if KnownPostgresType::Citext == $nt {
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
        small_int(KnownPostgresType::SmallInt) => Int32,
        integer(KnownPostgresType::Integer) => Int32,
        big_int(KnownPostgresType::BigInt) => Int64,
        nt_decimal(KnownPostgresType::Decimal(Some((4, 4)))) => Numeric,
        money(KnownPostgresType::Money) => Numeric,
        inet(KnownPostgresType::Inet) => Text,
        oid(KnownPostgresType::Oid) => Int64,
        citext(KnownPostgresType::Citext) => Text,
        real(KnownPostgresType::Real) => Float,
        double(KnownPostgresType::DoublePrecision) => Double,
        var_char(KnownPostgresType::VarChar(Some(255))) => Text,
        char(KnownPostgresType::Char(Some(255))) => Text,
        text(KnownPostgresType::Text) => Text,
        byte(KnownPostgresType::ByteA) => Bytes,
        timestamp(KnownPostgresType::Timestamp(Some(1))) => DateTime,
        timestamptz(KnownPostgresType::Timestamptz(Some(1))) => DateTime,
        date(KnownPostgresType::Date) => Date,
        time(KnownPostgresType::Time(Some(1))) => Time,
        timetz(KnownPostgresType::Timetz(Some(1))) => Time,
        boolean(KnownPostgresType::Boolean) => Boolean,
        bit(KnownPostgresType::Bit(Some(1))) => Text,
        var_bit(KnownPostgresType::VarBit(Some(1))) => Text,
        uuid(KnownPostgresType::Uuid) => Uuid,
        xml(KnownPostgresType::Xml) => Xml,
        json(KnownPostgresType::Json) => Json,
        json_b(KnownPostgresType::JsonB) => Json,
    }
}

mod crdb {
    use super::*;

    #[test_connector(tags(CockroachDb))]
    fn named_expr(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT 1 + 1 as \"add\";",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "add",
                        typ: "bigint",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT 1 + 1 as \"add\";")
            .send_sync()
            .expect_result(expected)
    }

    #[test_connector(tags(CockroachDb))]
    fn mixed_named_expr(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT \"int\" + 1 as \"add\" FROM \"model\";",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "add",
                        typ: "bigint",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT \"int\" + 1 as \"add\" FROM \"model\";")
            .send_sync()
            .expect_result(expected)
    }

    #[test_connector(tags(CockroachDb))]
    fn mixed_unnamed_expr(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            ConnectorErrorImpl {
                user_facing_error: None,
                message: Some(
                    "Invalid input provided to query: Invalid column name '?column?' for index 0. Your SQL query must explicitly alias that column name.",
                ),
                source: None,
                context: SpanTrace [],
            }
            Invalid input provided to query: Invalid column name '?column?' for index 0. Your SQL query must explicitly alias that column name.

        "#]];

        expected.assert_debug_eq(
            &api.introspect_sql("test_1", "SELECT \"int\" + 1 FROM \"model\";")
                .send_unwrap_err(),
        );
    }

    #[test_connector(tags(CockroachDb))]
    fn mixed_expr_cast(api: TestApi) {
        api.schema_push(SIMPLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT CAST(\"int\" + 1 as int) FROM model;",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "int8",
                        typ: "bigint",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT CAST(\"int\" + 1 as int) FROM model;")
            .send_sync()
            .expect_result(expected)
    }

    #[test_connector(tags(CockroachDb))]
    fn subquery(api: TestApi) {
        api.schema_push(SIMPLE_NULLABLE_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT int, foo.int, foo.string FROM (SELECT * FROM model) AS foo",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "int",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "int",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "string",
                        typ: "string",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql(
            "test_1",
            "SELECT int, foo.int, foo.string FROM (SELECT * FROM model) AS foo",
        )
        .send_sync()
        .expect_result(expected)
    }

    #[test_connector(tags(CockroachDb))]
    fn left_join(api: TestApi) {
        api.schema_push(RELATION_SCHEMA).send().assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "SELECT parent.id as parentId, parent.nullable as parentNullable, child.id as childId, child.nullable as childNullable FROM parent LEFT JOIN child ON parent.id = child.parent_id",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "parentid",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "parentnullable",
                        typ: "string",
                        nullable: true,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "childid",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "childnullable",
                        typ: "string",
                        nullable: true,
                    },
                ],
            }
        "#]];

        api.introspect_sql("test_1", "SELECT parent.id as parentId, parent.nullable as parentNullable, child.id as childId, child.nullable as childNullable FROM parent LEFT JOIN child ON parent.id = child.parent_id")
        .send_sync()
        .expect_result(expected)
    }

    // test nullability inference for various joins
    #[test_connector(tags(CockroachDb))]
    fn outer_join(api: TestApi) {
        api.schema_push(
            "model products {
                    product_no Int     @id
                    name       String?
                }

                model tweet {
                    id   Int    @id @default(autoincrement())
                    text String
                }",
        )
        .send()
        .assert_green();

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_1",
                source: "select tweet.id from (values (null)) vals(val) inner join tweet on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                ],
            }
        "#]];

        // inner join, nullability should not be overridden
        api.introspect_sql(
            "test_1",
            "select tweet.id from (values (null)) vals(val) inner join tweet on false",
        )
        .send_sync()
        .expect_result(expected);

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_2",
                source: "select tweet.id from (values (null)) vals(val) left join tweet on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                ],
            }
        "#]];

        // tweet.id is marked NOT NULL but it's brought in from a left-join here
        // which should make it nullable
        api.introspect_sql(
            "test_2",
            "select tweet.id from (values (null)) vals(val) left join tweet on false",
        )
        .send_sync()
        .expect_result(expected);

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_3",
                source: "select tweet1.id, tweet2.id from tweet tweet1 left join tweet tweet2 on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                ],
            }
        "#]];

        // make sure we don't mis-infer for the outer half of the join
        api.introspect_sql(
            "test_3",
            "select tweet1.id, tweet2.id from tweet tweet1 left join tweet tweet2 on false",
        )
        .send_sync()
        .expect_result(expected);

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_4",
                source: "select tweet1.id, tweet2.id from tweet tweet1 right join tweet tweet2 on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                ],
            }
        "#]];

        // right join, nullability should be inverted
        api.introspect_sql(
            "test_4",
            "select tweet1.id, tweet2.id from tweet tweet1 right join tweet tweet2 on false",
        )
        .send_sync()
        .expect_result(expected);

        let expected = expect![[r#"
            IntrospectSqlQueryOutput {
                name: "test_5",
                source: "select tweet1.id, tweet2.id from tweet tweet1 full join tweet tweet2 on false",
                documentation: None,
                parameters: [],
                result_columns: [
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                    IntrospectSqlQueryColumnOutput {
                        name: "id",
                        typ: "int",
                        nullable: false,
                    },
                ],
            }
        "#]];

        // right join, nullability should be inverted
        api.introspect_sql(
            "test_5",
            "select tweet1.id, tweet2.id from tweet tweet1 full join tweet tweet2 on false",
        )
        .send_sync()
        .expect_result(expected);
    }

    macro_rules! test_native_types_crdb {
    (
        $($test_name:ident($nt:expr) => $ct:ident,)*
    ) => {
        $(
            paste::paste! {
                #[test_connector(tags(CockroachDb))]
                fn $test_name(api: TestApi) {
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
