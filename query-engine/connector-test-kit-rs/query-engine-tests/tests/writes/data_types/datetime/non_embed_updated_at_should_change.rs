use query_engine_tests::*;

#[test_suite(schema(schema))]
mod non_embed_updated_at {
    use indoc::indoc;
    use query_engine_tests::run_query_json;
    use std::time::Duration;
    use tokio::time::sleep;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Top {
              #id(id, String, @id)
              top       String   @unique
              createdAt DateTime @default(now())
              updatedAt DateTime @updatedAt

              bottomId  String?  @unique
              bottom    Bottom?  @relation(fields: [bottomId], references: [id])
            }

            model Bottom {
              #id(id, String, @id)
              bottom    String   @unique
              top       Top?
              createdAt DateTime @default(now())
              updatedAt DateTime @updatedAt
            }"#
        };

        schema.to_owned()
    }

    // "Updating a nested data item" should "change it's updatedAt value"
    #[connector_test]
    async fn update_nested_item(runner: Runner) -> TestResult<()> {
        let created = run_query_json!(
            runner,
            r#"mutation {createOneTop(data: { id: "1", top: "top2", bottom: {create:{id: "1", bottom: "Bottom2"}} }) {bottom{updatedAt}}}"#
        );

        // We have to wait a bit to avoid test flakiness due to the finite precision of the clock
        sleep(Duration::from_millis(50)).await;

        let updated = run_query_json!(
            runner,
            r#"mutation {
                updateOneTop(
                  where: { top: "top2" }
                  data: { bottom: { update:{ bottom: { set: "bottom20" }}}}
                ) {
                  bottom{
                    updatedAt
                  }
                }
            }"#
        );

        let updated_at = &created["data"]["createOneTop"]["bottom"]["updatedAt"];
        let changed_updated_at = &updated["data"]["updateOneTop"]["bottom"]["updatedAt"];

        assert_ne!(updated_at, changed_updated_at);

        Ok(())
    }

    // "Upserting a nested data item" should "change it's updatedAt value"
    #[connector_test]
    async fn upsert_nested_item(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {createOneTop(data: { id: "1", top: "top4", bottom: {create:{id: "1", bottom: "Bottom4"}} }) {bottom{updatedAt}}}"#
        );
        let updated_at = &res["data"]["createOneTop"]["bottom"]["updatedAt"];

        // We have to wait a bit to avoid test flakiness due to the finite precision of the clock
        sleep(Duration::from_millis(50)).await;

        let res_2 = run_query_json!(
            &runner,
            r#"mutation {
                updateOneTop(
                  where: { top: "top4" }
                  data: { bottom: { upsert:{ create:{ bottom: "Should not matter" }, update:{ bottom: { set: "Bottom40" }}}}}
                ) {
                  bottom{
                    updatedAt
                  }
                }
            }"#
        );
        let changed_updated_at = &res_2["data"]["updateOneTop"]["bottom"]["updatedAt"];

        assert_ne!(updated_at, changed_updated_at);

        Ok(())
    }
}
