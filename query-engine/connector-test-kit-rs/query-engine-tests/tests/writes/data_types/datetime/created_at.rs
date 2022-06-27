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
        let res = run_query_json!(
            runner,
            r#"mutation {
              createOneTestModel(data: {
                id: 1,
                children: {
                  create: [{id: 1}, {id: 2}, {id: 3}, {id: 4}]
                }
              }) {
                created_dt
                updated_dt,
                children {
                  created_dt
                  updated_dt
                }
              }
            }"#,
            &["data", "createOneTestModel"]
        );

        let created_dt = res["created_dt"].to_string();

        assert_eq!(res["updated_dt"].to_string(), created_dt);

        let children = res["children"].as_array().unwrap();

        for child in children {
            assert_eq!(child["created_dt"].to_string(), created_dt);
            assert_eq!(child["updated_dt"].to_string(), created_dt);
        }

        Ok(())
    }
}
