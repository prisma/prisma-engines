use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(ScalarLists))]
mod updated_at {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};
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
              createdAt DateTime @default(now())
              updatedAt DateTime @updatedAt

              top       Top?
            }

            model List {
              #id(id, String, @id)
              list      String   @unique
              ints      Int[]
              createdAt DateTime @default(now())
              updatedAt DateTime @updatedAt
            }"#
        };

        schema.to_owned()
    }

    // "Updating a data item" should "change it's updatedAt value"
    #[connector_test]
    async fn update_should_change_updated_at(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {createOneTop(data: { id: "1", top: "top1" }) {updatedAt}}"#
        );
        let updated_at = &res["data"]["createOneTop"]["updatedAt"];

        // We have to wait a bit to avoid test flakiness due to the finite precision of the clock
        sleep(Duration::from_millis(50)).await;

        let res_2 = run_query_json!(
            &runner,
            r#"mutation {
                updateOneTop(
                  where: { top: "top1" }
                  data: { top: { set: "top10" }}
                ) {
                  updatedAt
                }
            }"#
        );
        let changed_updated_at = &res_2["data"]["updateOneTop"]["updatedAt"];

        assert_ne!(updated_at, changed_updated_at);

        Ok(())
    }

    // "Upserting a data item" should "change it's updatedAt value"
    #[connector_test]
    async fn upsert_should_change_updated_at(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {createOneTop(data: { id: "1", top: "top3" }) {updatedAt}}"#
        );
        let updated_at = &res["data"]["createOneTop"]["updatedAt"];

        // We have to wait a bit to avoid test flakiness due to the finite precision of the clock
        sleep(Duration::from_millis(50)).await;

        let res_2 = run_query_json!(
            &runner,
            r#"mutation {
                    upsertOneTop(
                      where: { top: "top3" }
                      update: { top: { set: "top30" }}
                      create: { top: "Should not matter" }
                    ) {
                      updatedAt
                    }
                }"#
        );
        let changed_updated_at = &res_2["data"]["upsertOneTop"]["updatedAt"];

        assert_ne!(updated_at, changed_updated_at);

        Ok(())
    }

    // "UpdateMany a data item" should "change it's updatedAt value"
    #[connector_test]
    async fn update_many_should_change_updated_at(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {createOneTop(data: { id: "1", top: "top5" }) {updatedAt}}"#
        );
        let updated_at = &res["data"]["createOneTop"]["updatedAt"];

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateManyTop(
              where: { top: { equals: "top5" }}
              data: { top: { set: "top50" }}
            ) {
              count
            }
          }"#),
          @r###"{"data":{"updateManyTop":{"count":1}}}"###
        );

        // We have to wait a bit to avoid test flakiness due to the finite precision of the clock
        sleep(Duration::from_millis(50)).await;

        let res_2 = run_query_json!(
            &runner,
            r#"query {
              findUniqueTop(where: { top: "top50" }) {
                updatedAt
              }
            }"#
        );
        let changed_updated_at = &res_2["data"]["findUniqueTop"]["updatedAt"];

        assert_ne!(updated_at, changed_updated_at);

        Ok(())
    }

    // "Updating scalar list values" should "change updatedAt values"
    #[connector_test]
    async fn update_sclr_list_should_change_updt_at(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {createOneList(data: { id: "1", list: "test" }) {updatedAt}}"#
        );
        let updated_at = &res["data"]["createOneList"]["updatedAt"];

        // We have to wait a bit to avoid test flakiness due to the finite precision of the clock
        sleep(Duration::from_millis(50)).await;

        let res_2 = run_query_json!(
            &runner,
            r#"mutation {
                updateOneList(
                  where: { list: "test" }
                  data: { ints: {set: [1,2,3]}}
                ) {
                  updatedAt
                  ints
                }
          }"#
        );
        let changed_updated_at = &res_2["data"]["updatedOneList"]["updatedAt"];

        assert_ne!(updated_at, changed_updated_at);

        Ok(())
    }
}
