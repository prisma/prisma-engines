use query_engine_tests::*;

#[test_suite]
mod atomic_number_ops {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              uniq Int           @unique
              rel  RelatedModel?
            }

            model RelatedModel {
             #id(id, Int, @id)
             field String
             tm_id Int @unique
             tm    TestModel @relation(fields: [tm_id], references: [id])
            }"#
        };

        schema.to_owned()
    }

    // "An updateOne mutation with number operations on the top and updates on the child (inl. child)" should "handle id changes correctly"
    #[connector_test(schema(schema_1), capabilities(UpdateableId))]
    async fn update_number_ops_on_child(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
          createOneTestModel(
            data: {
              id: 1
              uniq: 2
              rel: { create: { id: 1, field: "field" } }
            }
          ) {
            id
          }
        }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { uniq: 2 }
              data: {
                id: { increment: 1 }
                uniq: { multiply: 3 }
                rel: {
                  update: {
                    field: { set: "updated" }
                  }
                }
              }
            ){
              rel {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"rel":{"id":1}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { id: 2 }
              data: {
                id: { increment: 1 }
                uniq: { multiply: 3 }
                rel: {
                  update: {
                    field: { set: "updated 2" }
                  }
                }
              }
            ){
              rel {
                id
                field
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"rel":{"id":1,"field":"updated 2"}}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              uniq   Int          @unique
              rel_id Int
              rel    RelatedModel @relation(fields: [rel_id], references: [id])
            }

            model RelatedModel {
             #id(id, Int, @id)
             field String
             test  TestModel[]
            }"#
        };

        schema.to_owned()
    }

    //"An updateOne mutation with number operations on the top and updates on the child (inl. parent)" should "handle id changes correctly"
    #[connector_test(schema(schema_2), capabilities(UpdateableId))]
    async fn update_number_ops_on_parent(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
          createOneTestModel(
            data: {
              id: 1
              uniq: 2
              rel: { create: { id: 1, field: "field" } }
            }
          ) {
            id
          }
        }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { uniq: 2 }
              data: {
                id: { increment: 1 }
                uniq: { multiply: 3 }
                rel: {
                  update: {
                    field: { set: "updated" }
                  }
                }
              }
            ){
              rel {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"rel":{"id":1}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { id: 2 }
              data: {
                id: { increment: 1 }
                uniq: { multiply: 3 }
                rel: {
                  update: {
                    field: { set: "updated 2" }
                  }
                }
              }
            ){
              rel {
                id
                field
              }
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"rel":{"id":1,"field":"updated 2"}}}}"###
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              rel RelatedModel?
            }

            model RelatedModel {
             #id(id, Int, @id)
             optInt   Int?
             optFloat Float?
             tm_id    Int @unique
             tm       TestModel @relation(fields: [tm_id], references: [id])
            }"#
        };

        schema.to_owned()
    }

    // "A nested updateOne mutation" should "correctly apply all number operations for Int"
    #[connector_test(schema(schema_3), exclude(CockroachDb))]
    async fn nested_update_int_ops(runner: Runner) -> TestResult<()> {
        create_test_model(&runner, 1, None, None).await?;
        create_test_model(&runner, 2, Some(3), None).await?;

        // Increment
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "increment", "10").await?,
          @r###"{"optInt":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "increment", "10").await?,
          @r###"{"optInt":13}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "decrement", "10").await?,
          @r###"{"optInt":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "decrement", "10").await?,
          @r###"{"optInt":3}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "multiply", "2").await?,
          @r###"{"optInt":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "multiply", "2").await?,
          @r###"{"optInt":6}"###
        );

        // Divide
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "divide", "3").await?,
          @r###"{"optInt":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "divide", "3").await?,
          @r###"{"optInt":2}"###
        );

        // Set
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "set", "5").await?,
          @r###"{"optInt":5}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "set", "5").await?,
          @r###"{"optInt":5}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "set", "null").await?,
          @r###"{"optInt":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "set", "null").await?,
          @r###"{"optInt":null}"###
        );

        Ok(())
    }

    // CockroachDB does not support the "divide" operator as is.
    // See https://github.com/cockroachdb/cockroach/issues/41448.
    #[connector_test(schema(schema_3), only(CockroachDb))]
    async fn nested_update_int_ops_cockroach(runner: Runner) -> TestResult<()> {
        create_test_model(&runner, 1, None, None).await?;
        create_test_model(&runner, 2, Some(3), None).await?;

        // Increment
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "increment", "10").await?,
          @r###"{"optInt":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "increment", "10").await?,
          @r###"{"optInt":13}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "decrement", "10").await?,
          @r###"{"optInt":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "decrement", "10").await?,
          @r###"{"optInt":3}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "multiply", "2").await?,
          @r###"{"optInt":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "multiply", "2").await?,
          @r###"{"optInt":6}"###
        );

        // Set
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "set", "5").await?,
          @r###"{"optInt":5}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "set", "5").await?,
          @r###"{"optInt":5}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optInt", "set", "null").await?,
          @r###"{"optInt":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optInt", "set", "null").await?,
          @r###"{"optInt":null}"###
        );

        Ok(())
    }

    // "A nested updateOne mutation" should "correctly apply all number operations for Int"
    #[connector_test(schema(schema_3), exclude(MongoDb))]
    async fn nested_update_float_ops(runner: Runner) -> TestResult<()> {
        create_test_model(&runner, 1, None, None).await?;
        create_test_model(&runner, 2, None, Some("5.5")).await?;

        // Increment
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "increment", "4.6").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "increment", "4.6").await?,
          @r###"{"optFloat":10.1}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "decrement", "4.6").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "decrement", "4.6").await?,
          @r###"{"optFloat":5.5}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "multiply", "2").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "multiply", "2").await?,
          @r###"{"optFloat":11}"###
        );

        // Divide
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "divide", "2").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "divide", "2").await?,
          @r###"{"optFloat":5.5}"###
        );

        // Set
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "set", "5.1").await?,
          @r###"{"optFloat":5.1}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "set", "5.1").await?,
          @r###"{"optFloat":5.1}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "set", "null").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "set", "null").await?,
          @r###"{"optFloat":null}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_3), only(MongoDb))]
    async fn nested_update_float_ops_mongo(runner: Runner) -> TestResult<()> {
        create_test_model(&runner, 1, None, None).await?;
        create_test_model(&runner, 2, None, Some("5.5")).await?;

        // Increment
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "increment", "4.6").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "increment", "4.6").await?,
          @r###"{"optFloat":10.1}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "decrement", "4.6").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "decrement", "4.6").await?,
          @r###"{"optFloat":5.5}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "multiply", "2").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "multiply", "2").await?,
          @r###"{"optFloat":11}"###
        );

        // Divide
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "divide", "2").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "divide", "2").await?,
          @r###"{"optFloat":5.5}"###
        );

        // Set
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "set", "5.1").await?,
          @r###"{"optFloat":5.1}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "set", "5.1").await?,
          @r###"{"optFloat":5.1}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 1, "optFloat", "set", "null").await?,
          @r###"{"optFloat":null}"###
        );
        insta::assert_snapshot!(
          query_nested_number_ops(&runner, 2, "optFloat", "set", "null").await?,
          @r###"{"optFloat":null}"###
        );

        Ok(())
    }

    async fn create_test_model(
        runner: &Runner,
        id: u32,
        opt_int: Option<u32>,
        opt_float: Option<&str>,
    ) -> TestResult<()> {
        let f = opt_float.unwrap_or("null");
        let i = opt_int.map(|i| i.to_string()).unwrap_or_else(|| "null".to_string());

        run_query!(
            runner,
            format!(
                r#"mutation {{
                  createOneTestModel(
                    data: {{
                      id: {id}
                      rel: {{
                        create: {{
                          id: {id}
                          optInt: {i}
                          optFloat: {f}
                        }}
                      }}
                    }}
                  ) {{
                    id
                  }}
                }}"#
            )
        );

        Ok(())
    }

    async fn query_nested_number_ops(
        runner: &Runner,
        id: u32,
        field: &str,
        op: &str,
        value: &str,
    ) -> TestResult<String> {
        let res = run_query_json!(
            runner,
            format!(
                r#"mutation {{
                  updateOneTestModel(
                    where: {{ id: {id} }}
                    data: {{ rel: {{ update: {{ {field}: {{ {op}: {value} }}}}}}}}
                  ){{
                    rel {{
                      {field}
                    }}
                  }}
                }}"#
            ),
            &["data", "updateOneTestModel", "rel"]
        );

        Ok(res.to_string())
    }
}
