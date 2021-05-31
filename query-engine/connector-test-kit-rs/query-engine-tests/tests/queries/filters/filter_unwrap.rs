use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod filter_unwrap {
    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Item {
                #id(id, String, @id, @default(cuid()))
                name     String?   @unique
                subItems SubItem[]
            }

            model SubItem {
                #id(id, String, @id, @default(cuid()))
                name    String? @unique
                item_id String?

                item Item? @relation(fields: [item_id], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn many_filter(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc! {r#"
              mutation {
                createOneItem(
                  data: { name: "Top", subItems: { create: [{ name: "TEST1" }, { name: "TEST2" }] } }
                ) {
                  name
                  subItems {
                    name
                  }
                }
              }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! {r#"
              mutation {
                updateOneItem(
                  data: {
                    subItems: {
                      deleteMany: {
                        name: { in: ["TEST1", "TEST2"] }
                      }
                    }
                  }
                  where: { name: "Top" }
                ) {
                  name
                  subItems {
                    name
                  }
                }
              }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}
