use query_engine_tests::*;

#[test_suite(schema(schema))]
mod nested_create_many {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};
    fn schema() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              bs ModelB[]
            }

            model ModelB {
              #id(id, Int, @id)
              str1 String
              str2 String?
              str3 String? @default("SOME_DEFAULT")
              a_id Int?
              a    ModelA? @relation(fields: [a_id], references: [id])
            }"#
        };

        schema.to_owned()
    }

    // "A basic createMany on a create top level" should "work"
    #[connector_test]
    async fn create_many_on_create(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: 1,
              bs: {
                createMany: {
                  data: [
                    { id: 1, str1: "1", str2: "1", str3: "1"},
                    { id: 2, str1: "2",            str3: null},
                    { id: 3, str1: "1"},
                  ]
                }
              }
            }) {
              bs {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"bs":[{"id":1},{"id":2},{"id":3}]}}}"###
        );

        Ok(())
    }

    // "A basic createMany on a create top level" should "work"
    #[connector_test]
    async fn create_many_shorthand_on_create(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
                createOneModelA(data: {
                  id: 1,
                  bs: {
                    createMany: {
                      data: { id: 1, str1: "1", str2: "1", str3: "1"}
                    }
                  }
                }) {
                  bs {
                    id
                  }
                }
              }"#),
          @r###"{"data":{"createOneModelA":{"bs":[{"id":1}]}}}"###
        );

        Ok(())
    }

    // "Nested createMany" should "error on duplicates by default"
    // TODO(dom): Not working for mongo
    #[connector_test(exclude(MongoDb))]
    async fn nested_createmany_fail_dups(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
              createOneModelA(data: {
                id: 1,
                bs: {
                  createMany: {
                    data: [
                      { id: 1, str1: "1", str2: "1", str3: "1"},
                      { id: 1, str1: "2",            str3: null},
                    ]
                  }
                }
              }) {
                bs {
                  id
                }
              }
            }"#,
            2002,
            "Unique constraint failed"
        );

        Ok(())
    }

    // "Nested createMany" should "not error on duplicates with skipDuplicates true"
    // TODO(dom): Not working for mongo
    #[connector_test(exclude(Sqlite, SqlServer, MongoDb))]
    async fn no_error_on_dups_when_skip_dups(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: 1,
              bs: {
                createMany: {
                  skipDuplicates: true,
                  data: [
                    { id: 1, str1: "1", str2: "1", str3: "1"},
                    { id: 1, str1: "2",            str3: null},
                  ]
                }
              }
            }) {
              bs {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"bs":[{"id":1}]}}}"###
        );

        Ok(())
    }

    // Note: Checks were originally higher, but test method (command line args) blows up...
    // Covers: Batching by row number.
    // Each DB allows a certain amount of params per single query, and a certain number of rows.
    // We create 1000 nested records.
    // "Nested createMany" should "allow creating a large number of records (horizontal partitioning check)"
    #[connector_test]
    async fn allow_create_large_number_records(runner: Runner) -> TestResult<()> {
        let records: Vec<_> = (1..=1000).map(|i| format!(r#"{{ id: {i}, str1: "{i}" }}"#)).collect();

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  createOneModelA(data: {{
                    id: 1
                    bs: {{
                      createMany: {{
                        data: [{records}]
                      }}
                    }}
                  }}) {{
                    id
                  }}
                }}"#,
                records = records.join(",")
            )
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            aggregateModelB {
              _count {
                _all
              }
            }
          }"#),
          @r###"{"data":{"aggregateModelB":{"_count":{"_all":1000}}}}"###
        );

        Ok(())
    }
}
