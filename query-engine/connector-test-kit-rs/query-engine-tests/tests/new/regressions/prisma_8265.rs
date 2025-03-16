use query_engine_tests::*;

#[test_suite(schema(schema))]
mod mongodb {
    use indoc::indoc;
    use query_engine_tests::{run_query_json, Runner};
    use std::time::Duration;
    use tokio::time::sleep;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Order {
                #id(id, String, @id)
                order_lines OrderLine[]
            }

            model OrderLine {
                #id(id, String, @id)
                external_id String
                created_at  DateTime @default(now())
                updated_at  DateTime @default(now()) @updatedAt
                order_id    String
                order       Order    @relation(fields: [order_id], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn nested_update_many_timestamps(runner: Runner) -> TestResult<()> {
        let resp = run_query_json!(
            runner,
            r#"mutation { createOneOrder(data: {
                id: "order_1"
                order_lines: {
                    create: {
                        id: "order_line_1"
                        external_id: "1"
                    }
                }
            }) { order_lines { updated_at } }}"#
        );

        let updated_at = &resp["data"]["createOneOrder"]["order_lines"][0]["updated_at"];

        // We have to wait a bit to avoid test flakiness due to the finite precision of the clock
        sleep(Duration::from_millis(50)).await;

        let updated = run_query_json!(
            runner,
            r#"mutation {
                updateOneOrder(
                  where: { id: "order_1" }
                  data: {
                    order_lines: {
                      updateMany: {
                        where: { external_id: { not: { equals: "something" }}}
                        data: { external_id: { set: "changed" }}
                      }
                    }
                  }
                ) {
                  order_lines {
                    updated_at
                  }
                }
              }
              "#
        );

        let changed_updated_at = &updated["data"]["updateOneOrder"]["order_lines"][0]["updated_at"];
        assert_ne!(updated_at, changed_updated_at);

        Ok(())
    }
}
