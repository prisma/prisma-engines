use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod one2m {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)
              name String

              children Child[]
            }
            
            model Child {
              #id(id, Int, @id)
              name String

              parentId Int?
              parent Parent? @relation(fields: [parentId], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn simple(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyParent { id name children { id name }  } }"#),
          @r###""###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, name: "Bob", children: { create: [{ id: 1, name: "Hello!" }, { id: 2, name: "World!" }] } }"#,
        )
        .await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
