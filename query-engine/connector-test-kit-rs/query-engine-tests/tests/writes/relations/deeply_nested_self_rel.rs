use query_engine_tests::*;

#[test_suite]
mod deep_nested_rel {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name     String  @unique
              parentId Int?

              parent   User?  @relation(name: "Users", fields: [parentId], references: [id])
              children User[] @relation(name: "Users")
            }"#
        };

        schema.to_owned()
    }

    // TODO:Bring back sql server when cascading rules can be set!
    // "A deeply nested self relation create" should "be executed completely"
    #[connector_test(schema(schema_1), exclude(SqlServer))]
    async fn deep_nested_create_should_work(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneUser(
              data: {
                id: 1,
                name: "A"
                children: {
                  create: [
                    { id: 2, name: "B",
                      children: {
                        create: [{ id: 3, name: "C" }]
                      }
                  }]
                }
              }
            ) {
              name
              parent {name}
              children {
                name
                parent {name}
                children {
                  name
                  parent {name}
                  children {
                    parent {name}
                    id
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"createOneUser":{"name":"A","parent":null,"children":[{"name":"B","parent":{"name":"A"},"children":[{"name":"C","parent":{"name":"B"},"children":[]}]}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyUser { name } }"#),
          @r###"{"data":{"findManyUser":[{"name":"A"},{"name":"B"},{"name":"C"}]}}"###
        );

        Ok(())
    }
}
