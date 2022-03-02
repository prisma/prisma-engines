use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod validate {
    fn schema() -> String {
        let schema = indoc! {
            r#"model User {
                #id(id, String, @id, @default(cuid()))
                posts Post[]
              }

              model Post {
                #id(id, String, @id, @default(cuid()))
                author   User?   @relation(fields: [authorId], references: [id])
                authorId String? @map("author")
              }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn check(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
                createOnePost(data: {}) {
                    id
                    authorId
                }
            }"#
        );

        Ok(())
    }
}
