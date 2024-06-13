use indoc::indoc;
use query_engine_tests::*;

#[test_suite(only(SqlServer))]
mod string {
    fn schema_string() -> String {
        let schema = indoc! {
            r#"
            model Parent {
                #id(id, Int, @id)

                vChar String @test.VarChar
                vChar40 String @test.VarChar(40)
                vCharMax String @test.VarChar(max)
            }"#
        };

        schema.to_owned()
    }

    // Regression test for https://github.com/prisma/prisma/issues/17565
    #[connector_test(schema(schema_string))]
    async fn native_string(mut runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1,
              vChar: "0"
              vChar40: "0123456789012345678901234567890123456789"
              vCharMax: "0123456789"
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent {
            id
            vChar
            vChar40
            vCharMax
        }}"#),
          @r###"{"data":{"findManyParent":[{"id":1,"vChar":"0","vChar40":"0123456789012345678901234567890123456789","vCharMax":"0123456789"}]}}"###
        );

        // VARCHAR
        // Ensure the VarChar is casted to VARCHAR to avoid implicit coercion
        runner.clear_logs().await;
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent(where: { vChar: "0" }) {
              id
              vChar
              }}"#),
            @r###"{"data":{"findManyParent":[{"id":1,"vChar":"0"}]}}"###
        );
        assert!(runner
            .get_logs()
            .await
            .iter()
            .any(|log| log.contains("WHERE [string_native_string].[Parent].[vChar] = CAST(@P1 AS VARCHAR)")));

        // VARCHAR(40)
        runner.clear_logs().await;
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent(where: { vChar40: "0123456789012345678901234567890123456789" }) {
            id
            vChar40
          }}"#),
          @r###"{"data":{"findManyParent":[{"id":1,"vChar40":"0123456789012345678901234567890123456789"}]}}"###
        );

        // Ensure the VarChar(40) is casted to VARCHAR(40) to avoid implicit coercion
        assert!(runner
            .get_logs()
            .await
            .iter()
            .any(|log| log.contains("WHERE [string_native_string].[Parent].[vChar40] = CAST(@P1 AS VARCHAR(40))")));

        // VARCHAR(MAX)
        runner.clear_logs().await;
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent(where: { vCharMax: "0123456789" }) {
            id
            vCharMax
          }}"#),
          @r###"{"data":{"findManyParent":[{"id":1,"vCharMax":"0123456789"}]}}"###
        );

        // Ensure the VarChar is casted to VARCHAR(MAX) to avoid implicit coercion
        assert!(runner
            .get_logs()
            .await
            .iter()
            .any(|log| log.contains("WHERE [string_native_string].[Parent].[vCharMax] = CAST(@P1 AS VARCHAR(MAX))")));

        // Ensure it works as well with gt
        runner.clear_logs().await;
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent(where: { vChar40: { gt: "0" } }) { id vChar40 } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"vChar40":"0123456789012345678901234567890123456789"}]}}"###
        );
        assert!(runner
            .get_logs()
            .await
            .iter()
            .any(|log| log.contains("WHERE [string_native_string].[Parent].[vChar40] > CAST(@P1 AS VARCHAR(40))")));

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
