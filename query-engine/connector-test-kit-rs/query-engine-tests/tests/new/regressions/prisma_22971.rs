use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod prisma_22971 {
    fn schema() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id, @map("hello"))
              updatedAt String @default("now") @map("updated_at")
            
              postId Int?  @map("post")
              post   Post? @relation("User_post", fields: [postId], references: [id])
            }
            
            model Post {
              #id(id, Int, @id, @map("world"))
              updatedAt String @default("now") @map("up_at")
            
              from_User_post User[] @relation("User_post")
            }"#
        };

        schema.to_owned()
    }

    // Ensures that mapped fields are correctly resolved, even when there's a conflict between a scalar field name and a relation field name.
    #[connector_test]
    async fn test_22971(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOnePost(data: { id: 1 }) { id } }"#);
        run_query!(
            &runner,
            r#"mutation { createOneUser(data: { id: 1, postId: 1 }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser {
              id
              updatedAt
              post {
                id
                updatedAt
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":1,"updatedAt":"now","post":{"id":1,"updatedAt":"now"}}]}}"###
        );

        Ok(())
    }
}
