use query_engine_tests::*;

// mongodb has very specific constraint on id fields
// mssql fails with a multiple cascading referential actions paths error
#[test_suite(schema(schema), exclude(MongoDB, SqlServer))]
mod prisma_14696 {
    fn schema() -> String {
        include_str!("./prisma_14696.prisma").to_string()
    }

    #[connector_test]
    async fn create_does_not_panic(runner: Runner) -> TestResult<()> {
        // Create the user.
        {
            let response = runner
                .query(
                    r#"
                mutation { createOneUser(data: {
                    id: 1,
                    password: "1234batman",
                    description: "rad",
                    username: "1337kv",
                    googleId: "1",
                    avatar: "1",
                }) { id } }
            "#,
                )
                .await?;
            response.assert_success();
        }

        let final_request = r#"
            mutation {
              createOnePost(data: {
                id: 8,
                user: {
                  connect: {
                    id: 1
                  }
                }
                comments: {

                }
                likes: {

                }
              }) {
                id
                user {
                  id
                  googleId
                  username
                  avatar
                  description
                  password
                  createdAt
                }
                userId
                comments {
                  id
                  postId
                  userId
                  createdAt
                }
                likes {
                  id
                  userId
                  createdAt
                }
                createdAt
              }
            }
        "#;
        let response = runner.query(final_request).await?;
        response.assert_success();
        Ok(())
    }
}
