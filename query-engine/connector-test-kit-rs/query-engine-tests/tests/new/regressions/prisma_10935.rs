use query_engine_tests::*;

#[test_suite(schema(schema), only(MySql, TiDB))]
mod prisma_ref_integrity {
    fn schema() -> String {
        let schema = indoc! {
            r#"
              model User {
                id      Int       @id @default(autoincrement())
                email   String    @unique
                name    String?   @unique
                bio     String?
              }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(referential_integrity = "prisma")]
    async fn upserts_must_not_return_count(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          &run_upsert(&runner).await?,
          @r###"{"data":{"upsertOneUser":{"email":"test@test.com","name":"test","bio":"test"}}}"###
        );

        insta::assert_snapshot!(
          &run_upsert(&runner).await?,
          @r###"{"data":{"upsertOneUser":{"email":"test@test.com","name":"test","bio":"updated"}}}"###
        );

        Ok(())
    }

    async fn run_upsert(runner: &Runner) -> TestResult<String> {
        Ok(run_query!(
            runner,
            r#"
            mutation {
                upsertOneUser(where: {
                    name: "test"
                }, create: {
                    email: "test@test.com"
                    name: "test"
                    bio: "test"
                }, update: {
                    bio: "updated"
                }) {
                    email
                    name
                    bio
                }
            }
          "#
        ))
    }
}
