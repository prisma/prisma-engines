use query_engine_tests::*;

#[test_suite(schema(schema))]
mod basic_order_by {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model OrderTest {
                #id(id, Int, @id)
                uniqueField   Int    @unique
                nonUniqFieldA String
                nonUniqFieldB String
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn unique_asc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyOrderTest(orderBy: { uniqueField: asc }) {
              uniqueField
            }
          }"#),
          @r###"{"data":{"findManyOrderTest":[{"uniqueField":1},{"uniqueField":2},{"uniqueField":3},{"uniqueField":4},{"uniqueField":5},{"uniqueField":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn unique_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyOrderTest(orderBy: { uniqueField: desc }) {
              uniqueField
            }
          }"#),
          @r###"{"data":{"findManyOrderTest":[{"uniqueField":6},{"uniqueField":5},{"uniqueField":4},{"uniqueField":3},{"uniqueField":2},{"uniqueField":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn multiple_fields_basic(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyOrderTest(orderBy: [{ nonUniqFieldA: desc }, { uniqueField: desc}]) {
              nonUniqFieldA
              uniqueField
            }
          }"#),
          @r###"{"data":{"findManyOrderTest":[{"nonUniqFieldA":"C","uniqueField":6},{"nonUniqFieldA":"C","uniqueField":5},{"nonUniqFieldA":"B","uniqueField":4},{"nonUniqFieldA":"B","uniqueField":3},{"nonUniqFieldA":"A","uniqueField":2},{"nonUniqFieldA":"A","uniqueField":1}]}}"###
        );

        Ok(())
    }

    // Ordering by multiple fields should honor the order of the ordering fields defined in the query.
    #[connector_test]
    async fn multiple_fields_ordering(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // B ASC, A ASC, U ASC
        // A, A, 1
        // A, B, 4
        // B, A, 2
        // B, C, 5
        // C, B, 3
        // C, C, 6
        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyOrderTest(orderBy: [{ nonUniqFieldB: asc }, { nonUniqFieldA: asc }, { uniqueField: asc}]) {
              nonUniqFieldB
              nonUniqFieldA
              uniqueField
            }
          }"#),
          @r###"{"data":{"findManyOrderTest":[{"nonUniqFieldB":"A","nonUniqFieldA":"A","uniqueField":1},{"nonUniqFieldB":"A","nonUniqFieldA":"B","uniqueField":4},{"nonUniqFieldB":"B","nonUniqFieldA":"A","uniqueField":2},{"nonUniqFieldB":"B","nonUniqFieldA":"C","uniqueField":5},{"nonUniqFieldB":"C","nonUniqFieldA":"B","uniqueField":3},{"nonUniqFieldB":"C","nonUniqFieldA":"C","uniqueField":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn negative_cursor(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyOrderTest(take: -3, orderBy: { uniqueField: desc }) {
              uniqueField
            }
          }"#),
          @r###"{"data":{"findManyOrderTest":[{"uniqueField":3},{"uniqueField":2},{"uniqueField":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_order_objects(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            {
                findManyOrderTest(orderBy: {}) {
                    uniqueField
                }
            }"#),
          @r###"{"data":{"findManyOrderTest":[{"uniqueField":1},{"uniqueField":2},{"uniqueField":3},{"uniqueField":4},{"uniqueField":5},{"uniqueField":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"
            {
                findManyOrderTest(orderBy: [{}]) {
                    uniqueField
                }
            }"#),
          @r###"{"data":{"findManyOrderTest":[{"uniqueField":1},{"uniqueField":2},{"uniqueField":3},{"uniqueField":4},{"uniqueField":5},{"uniqueField":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"
            {
                findManyOrderTest(orderBy: [{}, {}]) {
                    uniqueField
                }
            }"#),
          @r###"{"data":{"findManyOrderTest":[{"uniqueField":1},{"uniqueField":2},{"uniqueField":3},{"uniqueField":4},{"uniqueField":5},{"uniqueField":6}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, uniqueField: 1, nonUniqFieldA: "A", nonUniqFieldB: "A"}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 2, uniqueField: 2, nonUniqFieldA: "A", nonUniqFieldB: "B"}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 3, uniqueField: 3, nonUniqFieldA: "B", nonUniqFieldB: "C"}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 4, uniqueField: 4, nonUniqFieldA: "B", nonUniqFieldB: "A"}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 5, uniqueField: 5, nonUniqFieldA: "C", nonUniqFieldB: "B"}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 6, uniqueField: 6, nonUniqFieldA: "C", nonUniqFieldB: "C"}"#,
        )
        .await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneOrderTest(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();

        Ok(())
    }
}
