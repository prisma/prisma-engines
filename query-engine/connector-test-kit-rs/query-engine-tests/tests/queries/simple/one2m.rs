use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod simple {
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
          @r###"{"data":{"findManyParent":[{"id":1,"name":"Bob","children":[{"id":1,"name":"Hello!"},{"id":2,"name":"World!"}]}]}}"###
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

#[test_suite(schema(schema))]
mod nested {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(parentId, Int, @id)
  
                children Child[]
              }
              
              model Child {
                #id(childId, Int, @id)
  
                parentId Int?
                parent Parent? @relation(fields: [parentId], references: [parentId])

                children GrandChild[]
              }
              
              model GrandChild {
                #id(grandChildId, Int, @id)

                parentId Int?
                parent Child? @relation(fields: [parentId], references: [childId])
              }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn vanilla(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyParent { parentId children { childId children { grandChildId } } } }"#),
          @r###"{"data":{"findManyParent":[{"parentId":1,"children":[{"childId":1,"children":[{"grandChildId":1},{"grandChildId":2}]},{"childId":2,"children":[{"grandChildId":3}]}]}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{
            parentId: 1,
            children: {
                create: [
                    { childId: 1, children: { create: [{ grandChildId: 1 }, { grandChildId: 2 }] }},
                    { childId: 2, children: { create: [{ grandChildId: 3 }] } }
                ]
            }
        }"#,
        )
        .await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {}) {{ parentId }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
