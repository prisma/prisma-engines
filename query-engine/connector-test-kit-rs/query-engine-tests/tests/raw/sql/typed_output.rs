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
          run_query_pretty!(&runner, fmt_query_raw(r#"SELECT * FROM "TestModel";"#, vec![])),
          @r###"
        {
          "data": {
            "queryRaw": {
              "columns": [
                "id",
                "string",
                "int",
                "bInt",
                "float",
                "bytes",
                "bool",
                "dt",
                "dec",
                "json",
                "string_list",
                "bInt_list"
              ],
              "types": [
                "int",
                "string",
                "int",
                "bigint",
                "double",
                "bytes",
                "bool",
                "datetime",
                "decimal",
                "json",
                "array",
                "array"
              ],
              "rows": [
                [
                  1,
                  "str",
                  42,
                  "9223372036854775807",
                  1.5432,
                  "AQID",
                  true,
                  "1900-10-10T01:10:10.001+00:00",
                  "123.4567891",
                  {
                    "a": "b"
                  },
                  [
                    [
                      "string",
                      "1"
                    ],
                    [
                      "string",
                      "a"
                    ],
                    [
                      "string",
                      "2"
                    ],
                    [
                      "string",
                      "123123213"
                    ]
                  ],
                  [
                    [
                      "bigint",
                      "-9223372036854775808"
                    ],
                    [
                      "bigint",
                      "9223372036854775807"
                    ]
                  ]
                ],
                [
                  2,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null
                ]
              ]
            }
          }
        }
        "###
        );

        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw(r#"SELECT 1 + 1;"#, vec![])),
          @r###"{"data":{"queryRaw":{"columns":["?column?"],"types":["int"],"rows":[[2]]}}}"###
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
            "queryRaw": {
              "columns": [
                "id",
                "string",
                "int",
                "bInt",
                "float",
                "bytes",
                "bool",
                "dt",
                "dec",
                "json"
              ],
              "types": [
                "int",
                "string",
                "int",
                "bigint",
                "double",
                "bytes",
                "int",
                "datetime",
                "decimal",
                "json"
              ],
              "rows": [
                [
                  1,
                  "str",
                  42,
                  "9223372036854775807",
                  1.5432,
                  "AQID",
                  1,
                  "1900-10-10T01:10:10.001+00:00",
                  "123.4567891",
                  {
                    "a": "b"
                  }
                ],
                [
                  2,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null
                ]
              ]
            }
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
            "queryRaw": {
              "columns": [
                "id",
                "string",
                "int",
                "bInt",
                "float",
                "bytes",
                "bool",
                "dt",
                "dec",
                "json"
              ],
              "types": [
                "int",
                "string",
                "int",
                "bigint",
                "double",
                "bytes",
                "int",
                "datetime",
                "decimal",
                "string"
              ],
              "rows": [
                [
                  1,
                  "str",
                  42,
                  "9223372036854775807",
                  1.5432,
                  "AQID",
                  1,
                  "1900-10-10T01:10:10.001+00:00",
                  "123.4567891",
                  "{\"a\":\"b\"}"
                ],
                [
                  2,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null
                ]
              ]
            }
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

    #[connector_test(schema(schema_sqlite), only(Sqlite), exclude(Sqlite("cfd1")))]
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
            "queryRaw": {
              "columns": [
                "id",
                "string",
                "int",
                "bInt",
                "float",
                "bytes",
                "bool",
                "dt",
                "dec"
              ],
              "types": [
                "int",
                "string",
                "int",
                "bigint",
                "double",
                "bytes",
                "bool",
                "datetime",
                "decimal"
              ],
              "rows": [
                [
                  1,
                  "str",
                  42,
                  "9223372036854775807",
                  1.5432,
                  "AQID",
                  true,
                  "1900-10-10T01:10:10.001+00:00",
                  "123.4567891"
                ],
                [
                  2,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null,
                  null
                ]
              ]
            }
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
