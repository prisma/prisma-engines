use query_engine_tests::*;

#[test_suite(schema(schema))]
mod inline_rel {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, String, @id)
              bool Boolean @default(true)
              b_id String?
              b    ModelB? @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, String, @id)
              model ModelA[]
            }"#
        };

        schema.to_owned()
    }

    // "Querying the scalar field that backs a relation and the relation itself" should "work"
    #[connector_test]
    async fn scalar_field_back_relation(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: "1" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA {
              id
              b_id
              b {
                id
              }
              bool
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":"1","b_id":null,"b":null,"bool":true}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneModelA(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
