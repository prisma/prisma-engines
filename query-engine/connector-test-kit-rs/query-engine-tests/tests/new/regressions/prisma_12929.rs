use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod mongodb {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Post {
                id       String @id @map("_id")
                author   User   @relation(fields: [authorId], references: [id])
                authorId String @map("author")

                @@map("posts")
            }

            model User {
                id      String @id @map("_id")
                posts   Post[]

                @@map("users")
            }
            "#
        };
        schema.to_owned()
    }

    // Checks that the relation field "author" is not picked over field "authorId", which maps to "author".
    #[connector_test]
    async fn no_field_confusion(runner: Runner) -> TestResult<()> {
        // User to connect to.
        runner
            .query(r#"mutation { createOneUser(data: { id: "foo" }) { id } }"#)
            .await?
            .assert_success();

        // This must succeed without erroring on the `author` field.
        assert_query!(
            runner,
            r#"
            mutation {
                createOnePost(data:{
                  id: "bar"
                  author: {
                    connect:{
                      id: "foo"
                    }
                  }
                }) {
                  id
                }
              }
            "#,
            r#"{"data":{"createOnePost":{"id":"bar"}}}"#
        );

        Ok(())
    }
}
