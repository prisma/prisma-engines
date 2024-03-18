use indoc::indoc;
use query_engine_tests::*;

#[test_suite(only(Postgres))]
mod null_list {
    use query_engine_tests::{fmt_query_raw, run_query, run_query_pretty};

    #[connector_test(schema(common_list_types), only(Postgres))]
    async fn null_scalar_lists(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id", "string", "int", "bInt", "float", "bytes", "bool", "dt") VALUES ($1, $2, $3, $4, $5, $6, $7, $8);"#,
                vec![
                    RawParam::from(1),
                    RawParam::array(vec![RawParam::from("hello"), RawParam::Null]),
                    RawParam::array(vec![RawParam::from(1337), RawParam::Null]),
                    RawParam::array(vec![RawParam::bigint(133737), RawParam::Null]),
                    RawParam::array(vec![RawParam::from(13.37), RawParam::Null]),
                    RawParam::array(vec![RawParam::bytes(&[1, 2, 3]), RawParam::Null]),
                    RawParam::array(vec![RawParam::from(true), RawParam::Null]),
                    RawParam::array(vec![
                        RawParam::try_datetime("1900-10-10T01:10:10.001Z")?,
                        RawParam::Null
                    ]),
                ],
            )
        );

        insta::assert_snapshot!(
          run_query_pretty!(&runner, fmt_query_raw(r#"SELECT * FROM "TestModel";"#, vec![])),
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
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "string",
                      "prisma__value": "hello"
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                },
                "int": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "int",
                      "prisma__value": 1337
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                },
                "bInt": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "bigint",
                      "prisma__value": "133737"
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                },
                "float": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "double",
                      "prisma__value": 13.37
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                },
                "bytes": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "bytes",
                      "prisma__value": "AQID"
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                },
                "bool": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "bool",
                      "prisma__value": true
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                },
                "dt": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "datetime",
                      "prisma__value": "1900-10-10T01:10:10.001+00:00"
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                }
              }
            ]
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
                    RawParam::array(vec![RawParam::from("127.0.0.1"), RawParam::Null]),
                    RawParam::array(vec![RawParam::from(123), RawParam::Null]),
                ],
            )
        );

        insta::assert_snapshot!(
          run_query_pretty!(&runner, fmt_query_raw(r#"SELECT * FROM "TestModel";"#, vec![])),
          @r###"
        {
          "data": {
            "queryRaw": [
              {
                "id": {
                  "prisma__type": "int",
                  "prisma__value": 1
                },
                "uuid": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "uuid",
                      "prisma__value": "936da01f-9abd-4d9d-80c7-02af85c822a8"
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                },
                "bit": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "string",
                      "prisma__value": "1"
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                },
                "inet": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "string",
                      "prisma__value": "127.0.0.1"
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                },
                "oid": {
                  "prisma__type": "array",
                  "prisma__value": [
                    {
                      "prisma__type": "bigint",
                      "prisma__value": "123"
                    },
                    {
                      "prisma__type": "null",
                      "prisma__value": null
                    }
                  ]
                }
              }
            ]
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
            @r###"{"data":{"queryRaw":[{"array_agg":{"prisma__type":"array","prisma__value":[{"prisma__type":"int","prisma__value":1},{"prisma__type":"null","prisma__value":null}]}}]}}"###
        );

        Ok(())
    }
}
