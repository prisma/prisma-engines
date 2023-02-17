use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod mongo_explicit_m2m {
    fn schema() -> String {
        r#"
            model User {
                id String @id @map("_id")
                catIds String[] @map("cat_ids")
                cats Cat[] @relation(fields: [catIds], references: [id])
            }

            model Cat {
                id String @id @map("_id")
                ownerIds String[] @map("owner_ids")
                owners User[] @relation(fields: [ownerIds], references: [id])
            }
        "#
        .to_owned()
    }

    #[connector_test]
    async fn connect(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(
                  data: {
                      id: "George"
                  }
                ) {
                  id
                }
              }              
          "#
        );
        run_query!(
            &runner,
            r#"mutation {
                createOneCat(
                  data: {
                      id: "Whiskers"
                  }
                ) {
                  id
                }
              }              
          "#
        );

        let result = run_query!(
            &runner,
            r#"mutation {
                updateOneUser(
                  where: {
                      id: "George"
                  }
                  data: {
                      cats: {
                          connect: [{ id: "Whiskers" }]
                      }
                  }
                ) {
                  id
                  cats { id }
                }
              }              
          "#
        );

        panic!("{:#?}", result);

        Ok(())
    }
}
