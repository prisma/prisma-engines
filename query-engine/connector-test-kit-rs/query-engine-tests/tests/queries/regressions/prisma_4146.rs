use query_engine_tests::*;

// validates fix for
// https://github.com/prisma/prisma/issues/4146

// Updating a many-to-many relationship using connect should update the
// `updatedAt` field on the side where the relationship is embedded.

#[test_suite(schema(schema))]
mod prisma_4146 {
    use indoc::indoc;
    use query_engine_tests::run_query;
    use std::time::Duration;
    use tokio::time::sleep;

    fn schema() -> String {
        let schema = indoc! {
            r#" model Account {
              #id(id, Int, @id)
              tokens     Token[]
              updatedAt  DateTime @updatedAt
            }

            model Token {
              #id(id, Int, @id)
              name         String
              account      Account? @relation(fields: [accountId], references: [id])
              accountId    Int?
              updatedAt    DateTime @updatedAt
            }"#
        };

        schema.to_owned()
    }

    // "Updating a list of fields over a connect bound" should "change the update fields tagged with @updatedAt"
    #[connector_test]
    async fn update_list_fields_connect_bound(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneAccount(data: { id: 1 }) {
                  id
                }
            }"#
        );

        let updated_at = run_query!(
            &runner,
            r#"mutation {
          createOneToken(data: { id: 2, name: "test" }) {
            updatedAt
          }
        }"#
        );
        let updated_at: serde_json::Value = serde_json::from_str(updated_at.as_str()).unwrap();
        let updated_at = &updated_at["data"]["createOneToken"]["updatedAt"].to_string();

        // We have to wait a bit to avoid test flakiness due to the finite precision of the clock
        sleep(Duration::from_millis(50)).await;

        let tokens = run_query!(
            &runner,
            r#"mutation {
          updateOneAccount(
            where: { id: 1 }
            data: { tokens: { connect: { id: 2 } } }
          ) {
            tokens {
              updatedAt
            }
          }
        }"#
        );
        let tokens: serde_json::Value = serde_json::from_str(tokens.as_str()).unwrap();
        let tokens = &tokens["data"]["updateOneAccount"]["tokens"][0]["updatedAt"].to_string();

        assert_ne!(updated_at, tokens);

        Ok(())
    }
}
