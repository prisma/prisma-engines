use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod relation_is_null {
    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Message {
                #id(id, String, @id, @default(cuid()))
                messageName String?
                image_id    String?
                image       Image?  @relation(fields: [image_id], references: [id])
            }

            model Image {
                #id(id, String, @id, @default(cuid()))
                imageName String?
                message   Message?
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn is_null(runner: &Runner) -> TestResult<()> {
        test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyImage(where: { message: { is: null }}) { imageName }}"#),
          @r###"{"data":{"findManyImage":[{"imageName":"image 2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyMessage(where: { image: { is: null }}) { messageName }}"#),
          @r###"{"data":{"findManyMessage":[{"messageName":"message 1"}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneMessage(data: { messageName: "message 1"}) { messageName }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneMessage(data: { messageName: "message 2", image: { create: { imageName: "image 1" }}}) { messageName }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneImage(data: { imageName: "image 2" }) { imageName }}"#)
            .await?
            .assert_success();

        Ok(())
    }
}
