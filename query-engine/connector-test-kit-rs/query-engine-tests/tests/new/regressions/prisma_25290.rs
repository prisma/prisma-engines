use query_engine_tests::*;

#[test_suite(schema(schema), only(Postgres, CockroachDb, Sqlite))]
mod prisma_25290 {
    fn schema() -> String {
        indoc! {
            r#"
            model User {
                id     Int    @id
                email  String @unique
                name   String?
                secret String? @ignore // This field should be ignored in query results
            }
            "#
        }
        .to_string()
    }

    // Test for issue #25290: Attempting a createManyAndReturn with an ignored field
    // should result in a GraphQL validation error. Prior to this issue the engine would panic
    // https://github.com/prisma/prisma/issues/25290
    #[connector_test]
    async fn cm_selecting_ignored_field_errors(runner: Runner) -> TestResult<()> {
        let result = runner
            .query(
                r#"
                mutation {
                  createManyUserAndReturn(data: [
                    { id: 1, email: "alice@prisma.io", name: "Alice" },
                    { id: 2, email: "bob@prisma.io", name: "Bob" }
                  ]) {
                    id
                    email
                    name
                    secret
                  }
                }
                "#,
            )
            .await?;

        result.assert_failure(2009, None);
        Ok(())
    }

    #[connector_test]
    async fn cm_does_not_return_ignored_fields(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createManyUserAndReturn(data: [{ id: 3, email: "charlie@prisma.io", name: "Charlie" }]) { id }}"#),
          @r###"{"data":{"createManyUserAndReturn":[{"id":3}]}}"###
        );
        Ok(())
    }
}
