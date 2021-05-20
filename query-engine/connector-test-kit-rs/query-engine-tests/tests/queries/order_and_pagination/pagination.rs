use indoc::indoc;
use query_engine_tests::*;
use std::cmp;

#[test_suite(schema(schema))]
mod pagination {
    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              field       String
              uniqueField String @unique
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn cursor_on_id(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6},{"id":7},{"id":8},{"id":9},{"id":10}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_id_ordering(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, orderBy: { id: desc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":4},{"id":3},{"id":2},{"id":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_id_order_desc_non_uniq(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, orderBy: { field: desc }) {
                id
                field
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":5,"field":"Field5"},{"id":6,"field":"Field5"},{"id":3,"field":"Field3"},{"id":4,"field":"Field3"},{"id":1,"field":"Field1"},{"id":2,"field":"Field1"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_id_end_of_records(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 10
              }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":10}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_id_first_record_reverse_order(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 1
              }, orderBy: { id: desc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_id_non_existing_cursor(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 999
              }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_on_unique(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                uniqueField: "Unique5"
              }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6},{"id":7},{"id":8},{"id":9},{"id":10}]}}"###
        );

        Ok(())
    }

    // Take only tests

    #[connector_test]
    async fn take_1(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
          query {
            findManyTestModel(take: 1) {
              id
            }
          }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn take_1_reverse_order(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(take: 1, orderBy: { id: desc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":10}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn take_0(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(take: 0) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn take_minus_one_without_cursor(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(take: -1, orderBy: { id: asc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":10}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn skip_returns_all_after_offset(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
          query {
            findManyTestModel(skip: 5, orderBy: { id: asc }) {
              id
            }
          }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7},{"id":8},{"id":9},{"id":10}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn skip_reversed_order(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(skip: 5, orderBy: { id: desc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":4},{"id":3},{"id":2},{"id":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn skipping_beyond_all_records(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(skip: 999) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn skip_0_records(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(skip: 0, orderBy: { id: asc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9},{"id":10}]}}"###
        );

        Ok(())
    }

    // Cursor + Take tests

    #[connector_test]
    async fn cursor_take_2(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, take: 2) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_take_minus_2(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, take: -2, orderBy: { id: asc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_last_record_take_2(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 10
              }, take: 2) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":10}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_first_record_take_minus_2(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 1
              }, take: -2) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_take_0(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 1
              }, take: 0) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_take_2_reverse_order(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, take: 2, orderBy: { id: desc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_take_minus_2_reverse_order(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, take: -2, orderBy: { id: desc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":5}]}}"###
        );

        Ok(())
    }

    // Cursor + take + skip tests

    #[connector_test]
    async fn cursor_take_2_skip_2(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, take: 2, skip: 2) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":7},{"id":8}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_take_minus_2_skip_2(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, take: -2, skip: 2, orderBy: { id: asc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn skip_to_end_with_take(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 9
              }, take: 2, skip: 2) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_take_0_skip_1(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 1
              }, skip: 1, take: 0) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_take_2_skip_2_reverse_order(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, take: 2, skip: 2, orderBy: { id: desc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_take_minus_2_skip_2_rev_order(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"
            query {
              findManyTestModel(cursor: {
                id: 5
              }, take: -2, skip: 2, orderBy: { id: desc }) {
                id
              }
            }
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":8},{"id":7}]}}"###
        );

        Ok(())
    }

    // Cursor + take + skip + multiple order by tests

    // TODO
    // #[connector_test]
    // async fn cursor_take_skip_multiple_stable_order(runner: &Runner) -> TestResult<()> {
    //     create_test_data(runner).await?;

    //     insta::assert_snapshot!(
    //       run_query!(runner, r#""#),
    //       @r###""###
    //     );

    //     Ok(())
    // }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        let n: [i32; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        for i in n.iter() {
            create_row(
                runner,
                format!(
                    "{{ id: {}, field: \"Field{}\", uniqueField: \"Unique{}\" }}",
                    i,
                    cmp::max(i - 1 + (i % 2), 0),
                    i
                )
                .as_str(),
            )
            .await?;
        }

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
