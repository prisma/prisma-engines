use indoc::indoc;
use query_engine_tests::*;

#[test_suite(capabilities(SqlQueryRaw))]
mod typed_output {
    use query_engine_tests::{fmt_query_raw, run_query, run_query_pretty};

    fn schema_pg() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                string      String?
                int         Int?
                bInt        BigInt?
                float       Float?
                bytes       Bytes?
                bool        Boolean?
                dt          DateTime?
                dec         Decimal?
                json        Json?
                string_list String[]
                bInt_list   BigInt[]
              }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_pg), only(Postgres))]
    async fn all_scalars_pg(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            string: "str",
            int: 42,
            bInt: "9223372036854775807",
            float: 1.5432,
            bytes: "AQID",
            bool: true,
            dt: "1900-10-10T01:10:10.001Z",
            dec: "123.45678910",
            json: "{\"a\": \"b\"}"
            string_list: ["1", "a", "2", "123123213"],
            bInt_list: ["-9223372036854775808", "9223372036854775807"]
        }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;

        insta::assert_snapshot!(
          run_query_pretty!(&runner, fmt_query_raw(r#"SELECT * FROM "typed_output_all_scalars_pg"."TestModel";"#, vec![])),
          @r###"
        {
          "data": {
            "queryRaw": [
              {
                "id": {
                  "prisma__type": "int",
                  "prisma__value": 1
                },
                "string": {
                  "prisma__type": "string",
                  "prisma__value": "str"
                },
                "int": {
                  "prisma__type": "int",
                  "prisma__value": 42
                },
                "bInt": {
                  "prisma__type": "bigint",
                  "prisma__value": "9223372036854775807"
                },
                "float": {
                  "prisma__type": "double",
                  "prisma__value": 1.5432
                },
                "bytes": {
                  "prisma__type": "bytes",
                  "prisma__value": "AQID"
                },
                "bool": {
                  "prisma__type": "bool",
                  "prisma__value": true
                },
                "dt": {
                  "prisma__type": "datetime",
                  "prisma__value": "1900-10-10T01:10:10.001+00:00"
                },
                "dec": {
                  "prisma__type": "decimal",
                  "prisma__value": "123.4567891"
                },
                "json": {
                  "prisma__type": "json",
                  "prisma__value": {
                    "a": "b"
                  }
                },
                "string_list": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "string",
                      "prisma__value": "1"
                    },
                    {
                      "prisma__type": "string",
                      "prisma__value": "a"
                    },
                    {
                      "prisma__type": "string",
                      "prisma__value": "2"
                    },
                    {
                      "prisma__type": "string",
                      "prisma__value": "123123213"
                    }
                  ]
                },
                "bInt_list": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "bigint",
                      "prisma__value": "-9223372036854775808"
                    },
                    {
                      "prisma__type": "bigint",
                      "prisma__value": "9223372036854775807"
                    }
                  ]
                }
              },
              {
                "id": {
                  "prisma__type": "int",
                  "prisma__value": 2
                },
                "string": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "int": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bInt": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "float": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bytes": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bool": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "dt": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "dec": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "json": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "string_list": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bInt_list": {
                  "prisma__type": "null",
                  "prisma__value": null
                }
              }
            ]
          }
        }
        "###
        );

        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw(r#"SELECT 1 + 1;"#, vec![])),
          @r###"{"data":{"queryRaw":[{"?column?":{"prisma__type":"int","prisma__value":2}}]}}"###
        );

        Ok(())
    }

    fn schema_mysql() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              string      String?
              int         Int?
              bInt        BigInt?
              float       Float?
              bytes       Bytes?
              bool        Boolean?
              dt          DateTime?
              dec         Decimal?
              json        Json?
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_mysql), only(MySql(5.7), MySql(8)))]
    async fn all_scalars_mysql(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            string: "str",
            int: 42,
            bInt: "9223372036854775807",
            float: 1.5432,
            bytes: "AQID",
            bool: true,
            dt: "1900-10-10T01:10:10.001Z",
            dec: "123.45678910",
            json: "{\"a\": \"b\"}"
          }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;

        insta::assert_snapshot!(
          run_query_pretty!(&runner, fmt_query_raw(r#"SELECT * FROM TestModel;"#, vec![])),
          @r###"
        {
          "data": {
            "queryRaw": [
              {
                "id": {
                  "prisma__type": "int",
                  "prisma__value": 1
                },
                "string": {
                  "prisma__type": "string",
                  "prisma__value": "str"
                },
                "int": {
                  "prisma__type": "int",
                  "prisma__value": 42
                },
                "bInt": {
                  "prisma__type": "bigint",
                  "prisma__value": "9223372036854775807"
                },
                "float": {
                  "prisma__type": "double",
                  "prisma__value": 1.5432
                },
                "bytes": {
                  "prisma__type": "bytes",
                  "prisma__value": "AQID"
                },
                "bool": {
                  "prisma__type": "int",
                  "prisma__value": 1
                },
                "dt": {
                  "prisma__type": "datetime",
                  "prisma__value": "1900-10-10T01:10:10.001+00:00"
                },
                "dec": {
                  "prisma__type": "decimal",
                  "prisma__value": "123.4567891"
                },
                "json": {
                  "prisma__type": "json",
                  "prisma__value": {
                    "a": "b"
                  }
                }
              },
              {
                "id": {
                  "prisma__type": "int",
                  "prisma__value": 2
                },
                "string": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "int": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bInt": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "float": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bytes": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bool": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "dt": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "dec": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "json": {
                  "prisma__type": "null",
                  "prisma__value": null
                }
              }
            ]
          }
        }
        "###
        );

        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw(r#"SELECT 1 + 1;"#, vec![])),
          @r###"{"data":{"queryRaw":[{"1 + 1":{"prisma__type":"bigint","prisma__value":"2"}}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_mysql), only(MySql("mariadb")))]
    async fn all_scalars_mariadb(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            string: "str",
            int: 42,
            bInt: "9223372036854775807",
            float: 1.5432,
            bytes: "AQID",
            bool: true,
            dt: "1900-10-10T01:10:10.001Z",
            dec: "123.45678910",
            json: "{\"a\": \"b\"}"
          }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;

        insta::assert_snapshot!(
          run_query_pretty!(&runner, fmt_query_raw(r#"SELECT * FROM TestModel;"#, vec![])),
          @r###"
        {
          "data": {
            "queryRaw": [
              {
                "id": {
                  "prisma__type": "int",
                  "prisma__value": 1
                },
                "string": {
                  "prisma__type": "string",
                  "prisma__value": "str"
                },
                "int": {
                  "prisma__type": "int",
                  "prisma__value": 42
                },
                "bInt": {
                  "prisma__type": "bigint",
                  "prisma__value": "9223372036854775807"
                },
                "float": {
                  "prisma__type": "double",
                  "prisma__value": 1.5432
                },
                "bytes": {
                  "prisma__type": "bytes",
                  "prisma__value": "AQID"
                },
                "bool": {
                  "prisma__type": "int",
                  "prisma__value": 1
                },
                "dt": {
                  "prisma__type": "datetime",
                  "prisma__value": "1900-10-10T01:10:10.001+00:00"
                },
                "dec": {
                  "prisma__type": "decimal",
                  "prisma__value": "123.4567891"
                },
                "json": {
                  "prisma__type": "string",
                  "prisma__value": "{\"a\":\"b\"}"
                }
              },
              {
                "id": {
                  "prisma__type": "int",
                  "prisma__value": 2
                },
                "string": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "int": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bInt": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "float": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bytes": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bool": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "dt": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "dec": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "json": {
                  "prisma__type": "null",
                  "prisma__value": null
                }
              }
            ]
          }
        }
        "###
        );

        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw(r#"SELECT 1 + 1;"#, vec![])),
          @r###"{"data":{"queryRaw":[{"1 + 1":{"prisma__type":"int","prisma__value":2}}]}}"###
        );

        Ok(())
    }

    fn schema_sqlite() -> String {
        let schema = indoc! {
            r#"model TestModel {
            #id(id, Int, @id)
            string      String?
            int         Int?
            bInt        BigInt?
            float       Float?
            bytes       Bytes?
            bool        Boolean?
            dt          DateTime?
            dec         Decimal?
          }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_sqlite), only(Sqlite))]
    async fn all_scalars_sqlite(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            string: "str",
            int: 42,
            bInt: "9223372036854775807",
            float: 1.5432,
            bytes: "AQID",
            bool: true,
            dt: "1900-10-10T01:10:10.001Z",
            dec: "123.45678910",
          }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;

        insta::assert_snapshot!(
          run_query_pretty!(&runner, fmt_query_raw(r#"SELECT * FROM TestModel;"#, vec![])),
          @r###"
        {
          "data": {
            "queryRaw": [
              {
                "id": {
                  "prisma__type": "int",
                  "prisma__value": 1
                },
                "string": {
                  "prisma__type": "string",
                  "prisma__value": "str"
                },
                "int": {
                  "prisma__type": "int",
                  "prisma__value": 42
                },
                "bInt": {
                  "prisma__type": "bigint",
                  "prisma__value": "9223372036854775807"
                },
                "float": {
                  "prisma__type": "double",
                  "prisma__value": 1.5432
                },
                "bytes": {
                  "prisma__type": "bytes",
                  "prisma__value": "AQID"
                },
                "bool": {
                  "prisma__type": "bool",
                  "prisma__value": true
                },
                "dt": {
                  "prisma__type": "datetime",
                  "prisma__value": "1900-10-10T01:10:10.001+00:00"
                },
                "dec": {
                  "prisma__type": "decimal",
                  "prisma__value": "123.4567891"
                }
              },
              {
                "id": {
                  "prisma__type": "int",
                  "prisma__value": 2
                },
                "string": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "int": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bInt": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "float": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bytes": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "bool": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "dt": {
                  "prisma__type": "null",
                  "prisma__value": null
                },
                "dec": {
                  "prisma__type": "null",
                  "prisma__value": null
                }
              }
            ]
          }
        }
        "###
        );

        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw(r#"SELECT 1 + 1;"#, vec![])),
          @r###"{"data":{"queryRaw":[{"1 + 1":{"prisma__type":"bigint","prisma__value":"2"}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
