use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod created_at {
    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              children   Child[]

              created_dt DateTime? @default(now())
              updated_dt DateTime? @default(now())
            }
            
            model Child {
              #id(id, Int, @id)
              test       TestModel?     @relation(fields: [testId], references: [id])
              testId     Int?

              created_dt DateTime? @default(now())
              updated_dt DateTime? @default(now())
            }  
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn created_at_should_stay_consistent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneTestModel(data: {
              id: 1,
              children: {
                createMany: {
                  data: [{id: 1}, {id: 2}, {id: 3}, {id: 4}]
                }
              }
            }) {
              created_dt
              updated_dt,
              children {
                created_dt
                updated_dt
              }
            }
          }"#),
          @r###"{"data":{"createOneTestModel":{"created_dt":"2022-06-23T14:56:00.304Z","updated_dt":"2022-06-23T14:56:00.304Z","children":[{"created_dt":"2022-06-23T14:56:00.304Z","updated_dt":"2022-06-23T14:56:00.304Z"},{"created_dt":"2022-06-23T14:56:00.304Z","updated_dt":"2022-06-23T14:56:00.304Z"},{"created_dt":"2022-06-23T14:56:00.304Z","updated_dt":"2022-06-23T14:56:00.304Z"},{"created_dt":"2022-06-23T14:56:00.304Z","updated_dt":"2022-06-23T14:56:00.304Z"}]}}}"###
        );

        Ok(())
    }
}
