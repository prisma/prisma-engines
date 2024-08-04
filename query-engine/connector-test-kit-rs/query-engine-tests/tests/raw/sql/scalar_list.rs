use indoc::indoc;
use query_engine_tests::*;

#[test_suite(only(Postgres, CockroachDb))]
mod scalar_list {
    use query_engine_tests::{fmt_query_raw, run_query, run_query_pretty};

    #[connector_test(schema(common_list_types))]
    async fn null_scalar_lists(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id", "string", "int", "bInt", "float", "bytes", "bool", "dt") VALUES ($1, $2, $3, $4, $5, $6, $7, $8);"#,
                vec![
                    RawParam::from(1),
                    RawParam::array(vec![RawParam::Null, RawParam::from("hello")]),
                    RawParam::array(vec![RawParam::from(1337), RawParam::Null]),
                    RawParam::array(vec![RawParam::Null, RawParam::bigint(133737)]),
                    RawParam::array(vec![RawParam::from(13.37), RawParam::Null]),
                    RawParam::array(vec![RawParam::Null, RawParam::bytes(&[1, 2, 3])]),
                    RawParam::array(vec![RawParam::from(true), RawParam::Null]),
                    RawParam::array(vec![
                        RawParam::Null,
                        RawParam::try_datetime("1900-10-10T01:10:10.001Z")?,
                    ]),
                ],
            )
        );

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
                "dt"
              ],
              "types": [
                "int",
                "string-array",
                "int-array",
                "bigint-array",
                "double-array",
                "bytes-array",
                "bool-array",
                "datetime-array"
              ],
              "rows": [
                [
                  1,
                  [
                    null,
                    "hello"
                  ],
                  [
                    1337,
                    null
                  ],
                  [
                    null,
                    "133737"
                  ],
                  [
                    13.37,
                    null
                  ],
                  [
                    null,
                    "AQID"
                  ],
                  [
                    true,
                    null
                  ],
                  [
                    null,
                    "1900-10-10T01:10:10.001+00:00"
                  ]
                ]
              ]
            }
          }
        }
        "###
        );

        Ok(())
    }

    fn native_list_types() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                uuid  String[] @test.Uuid
                bit   String[] @test.Bit(1)
                inet  String[] @test.Inet
                oid   Int[]    @test.Oid
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(native_list_types))]
    async fn null_native_type_lists(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id", "uuid", "bit", "inet", "oid") VALUES ($1, $2, $3, $4, $5);"#,
                vec![
                    RawParam::from(1),
                    RawParam::array(vec![
                        RawParam::from("936DA01F-9ABD-4D9D-80C7-02AF85C822A8"),
                        RawParam::Null
                    ]),
                    RawParam::array(vec![RawParam::from("1"), RawParam::Null]),
                    RawParam::array(vec![RawParam::Null, RawParam::from("127.0.0.1")]),
                    RawParam::array(vec![RawParam::Null, RawParam::from(123)]),
                ],
            )
        );

        insta::assert_snapshot!(
          run_query_pretty!(&runner, fmt_query_raw(r#"SELECT * FROM "TestModel";"#, vec![])),
          @r###"
        {
          "data": {
            "queryRaw": {
              "columns": [
                "id",
                "uuid",
                "bit",
                "inet",
                "oid"
              ],
              "types": [
                "int",
                "uuid-array",
                "string-array",
                "string-array",
                "bigint-array"
              ],
              "rows": [
                [
                  1,
                  [
                    "936da01f-9abd-4d9d-80c7-02af85c822a8",
                    null
                  ],
                  [
                    "1",
                    null
                  ],
                  [
                    null,
                    "127.0.0.1"
                  ],
                  [
                    null,
                    "123"
                  ]
                ]
              ]
            }
          }
        }
        "###
        );

        Ok(())
    }

    // Regression test for https://github.com/prisma/prisma/issues/11339
    #[connector_test(schema(common_nullable_types))]
    async fn prisma_11339(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            "mutation {
                createManyTestModel(data: [
                    { id: 1, int: 1 },
                    { id: 2 }
                ]) { count }
            }"
        );

        insta::assert_snapshot!(
            run_query!(&runner, fmt_query_raw(r#"SELECT ARRAY_AGG(int) FROM "TestModel";"#, vec![])),
            @r###"{"data":{"queryRaw":{"columns":["array_agg"],"types":["int-array"],"rows":[[[1,null]]]}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(common_list_types))]
    async fn empty_scalar_lists(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id", "string", "int", "bInt", "float", "bytes", "bool", "dt") VALUES ($1, $2, $3, $4, $5, $6, $7, $8);"#,
                vec![
                    RawParam::from(1),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                ],
            )
        );

        run_query!(
            &runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id", "string", "int", "bInt", "float", "bytes", "bool", "dt") VALUES ($1, $2, $3, $4, $5, $6, $7, $8);"#,
                vec![
                    RawParam::from(2),
                    RawParam::Array(vec![RawParam::Null, RawParam::from("hello")]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                    RawParam::Array(vec![]),
                ],
            )
        );

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
                "dt"
              ],
              "types": [
                "int",
                "string-array",
                "int-array",
                "bigint-array",
                "double-array",
                "bytes-array",
                "bool-array",
                "datetime-array"
              ],
              "rows": [
                [
                  1,
                  [],
                  [],
                  [],
                  [],
                  [],
                  [],
                  []
                ],
                [
                  2,
                  [
                    null,
                    "hello"
                  ],
                  [],
                  [],
                  [],
                  [],
                  [],
                  []
                ]
              ]
            }
          }
        }
        "###
        );

        Ok(())
    }

    #[connector_test(schema(common_list_types))]
    async fn null_only_scalar_lists(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id", "string", "int", "bInt", "float", "bytes", "bool", "dt") VALUES ($1, $2, $3, $4, $5, $6, $7, $8);"#,
                vec![
                    RawParam::from(1),
                    RawParam::Null,
                    RawParam::Null,
                    RawParam::Null,
                    RawParam::Null,
                    RawParam::Null,
                    RawParam::Null,
                    RawParam::Null,
                ],
            )
        );

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
                "dt"
              ],
              "types": [
                "int",
                "string-array",
                "int-array",
                "bigint-array",
                "double-array",
                "bytes-array",
                "bool-array",
                "datetime-array"
              ],
              "rows": [
                [
                  1,
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

        Ok(())
    }
}
