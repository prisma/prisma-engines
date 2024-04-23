use query_engine_tests::*;
use std::cmp;

#[test_suite(schema(schema))]
mod pagination {
    use indoc::indoc;
    use query_engine_tests::run_query;

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

    /***********************
     * Cursor only tests. *
     **********************/

    // should "return all records after and including the cursor"
    #[connector_test]
    async fn cursor_on_id(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // should "return all records after and including the cursor"
    #[connector_test]
    async fn cursor_id_ordering(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor (on ID) query with a descending order on a non-unique field" should "return all records after and including the cursor"
    #[connector_test]
    async fn cursor_id_order_desc_non_uniq(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor (on ID) query with an ascending order on a non-unique field" should "return all records after and including the cursor"
    #[connector_test]
    async fn cursor_id_order_asc_non_uniq(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
                query {
                  findManyTestModel(cursor: {
                    id: 5
                  }, orderBy: { field: asc }) {
                    id
                    field
                  }
                }
              "#),
          @r###"{"data":{"findManyTestModel":[{"id":5,"field":"Field5"},{"id":6,"field":"Field5"},{"id":7,"field":"Field7"},{"id":8,"field":"Field7"},{"id":9,"field":"Field9"},{"id":10,"field":"Field9"}]}}"###
        );

        Ok(())
    }

    // "A cursor (on ID) on the end of records" should "return only the last record"
    #[connector_test]
    async fn cursor_id_end_of_records(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor (on ID) on the first record but with reversed order" should "return only the first record"
    #[connector_test]
    async fn cursor_id_first_record_reverse_order(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor (on ID) on a non-existant cursor" should "return no records"
    #[connector_test]
    async fn cursor_id_non_existing_cursor(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor (on a unique)" should "work as well"
    #[connector_test]
    async fn cursor_on_unique(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    /*********************
     * Take only tests. *
     ********************/

    // "Taking 1" should "return only the first record"
    #[connector_test]
    async fn take_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "Taking 1 with reversed order" should "return only the last record"
    #[connector_test]
    async fn take_1_reverse_order(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "Taking 0" should "return no records"
    #[connector_test]
    async fn take_0(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "Taking -1 without a cursor" should "return the last record"
    #[connector_test]
    async fn take_minus_1_without_cursor(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    /*********************
     * Skip only tests. *
     ********************/

    // "A skip" should "return all records after the offset specified"
    #[connector_test]
    async fn skip_returns_all_after_offset(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A skip with order reversed" should "return all records after the offset specified"
    #[connector_test]
    async fn skip_reversed_order(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A skipping beyond all records" should "return no records"
    #[connector_test]
    async fn skipping_beyond_all_records(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "Skipping 0 records" should "return all records beginning from the first"
    #[connector_test]
    async fn skip_0_records(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    /*************************
     * Cursor + Take tests. *
     ************************/

    // "A cursor with take 2" should "return the cursor plus one record after the cursor"
    #[connector_test]
    async fn cursor_take_2(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor with take -2" should "return the cursor plus one record before the cursor"
    #[connector_test]
    async fn cursor_take_minus_2(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor on the last record with take 2" should "return only the cursor record"
    #[connector_test]
    async fn cursor_last_record_take_2(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor on the first record with take -2" should "return only the cursor record"
    #[connector_test]
    async fn cursor_first_record_take_minus_2(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor with take 0" should "return no records"
    #[connector_test]
    async fn cursor_take_0(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor with take 2 and reversed ordering" should "return the cursor record and the one before (in the original ordering)"
    #[connector_test]
    async fn cursor_take_2_reverse_order(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor with take -2 and reversed ordering" should "return the cursor record and the one after (in the original ordering)"
    #[connector_test]
    async fn cursor_take_minus_2_reverse_order(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    /********************************
     * Cursor + Take + Skip tests. *
     *******************************/

    // "A cursor with take 2 and skip 2" should "return 2 records after the next record after the cursor"
    #[connector_test]
    async fn cursor_take_2_skip_2(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor with take -2 and skip 2" should "return 2 records before the previous record of the cursor"
    #[connector_test]
    async fn cursor_take_minus_2_skip_2(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "Skipping to the end with take" should "return no records"
    #[connector_test]
    async fn skip_to_end_with_take(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor with take 0 and skip" should "return no records"
    #[connector_test]
    async fn cursor_take_0_skip_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor with take 2, skip 2 and reversed ordering" should "return 2 records before the record before the cursor (in the original ordering)"
    #[connector_test]
    async fn cursor_take_2_skip_2_reverse_order(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    // "A cursor with take -2, skip 2 and reversed ordering" should "return 2 records after the record before the cursor (in the original ordering)"
    #[connector_test]
    async fn cursor_take_minus_2_skip_2_rev_order(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
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

    /*************************************************
     * Cursor + Take + Skip + Multiple OrderBy tests. *
     * ************************************************/

    // "A cursor with take, skip and multiple order-bys with the orderBy combination stable" should "return the expected results generalized over more than 2 orderBys" in
    #[connector_test(schema(string_combination_unique))]
    async fn cursor_take_skip_multiple_stable_order(runner: Runner) -> TestResult<()> {
        // Test data:
        // All fields combined are a unique combination (guarantee stable ordering).
        //
        // ID   fieldA fieldB fieldC fieldD
        // 1 =>    A      B      C      D
        // 2 =>    A      A      A      B
        // 3 =>    B      B      B      B
        // 4 =>    B      B      B      C
        // 5 =>    C      C      B      A
        // 6 =>    C      C      D      C
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 1, fieldA: "A", fieldB: "B", fieldC: "C", fieldD: "D"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 2, fieldA: "A", fieldB: "A", fieldC: "A", fieldD: "B"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 3, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 4, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "C"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 5, fieldA: "C", fieldB: "C", fieldC: "B", fieldD: "A"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 6, fieldA: "C", fieldB: "C", fieldC: "D", fieldD: "C"}){ id }}"#
        );

        // Ordered: desc, ASC, ASC, DESC
        // 5 => C C B A
        // 6 => C C D C
        // 4 => B B B C <- cursor, skipped
        // 3 => B B B B <- take
        // 2 => A A A B <- take
        // 1 => A B C D
        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyTestModel(cursor: { id: 4 }, take: 2, skip: 1, orderBy: [{ fieldA: desc }, { fieldB: asc }, { fieldC: asc }, { fieldD: desc }]) {
              id
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":2}]}}"###
        );

        // Ordered (reverse from test #1): asc, DESC, DESC, ASC
        // 1 => A B C D
        // 2 => A A A B
        // 3 => B B B B
        // 4 => B B B C <- cursor, skipped
        // 6 => C C D C <- take
        // 5 => C C B A <- take
        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyTestModel(cursor: { id: 4 }, take: 2, skip: 1, orderBy: [{ fieldA: asc }, { fieldB: desc }, { fieldC: desc }, { fieldD: asc }]) {
              id
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":5}]}}"###
        );

        // Note: Negative takes reverse the order, the following tests check that.

        // >>> TEST #3, same order as 1, but gets reversed to test 2
        // Originally the query orders: desc, ASC, ASC, DESC. With -2 instead of 2, it wants to take:
        // 5 => C C B A <- take
        // 6 => C C D C <- take
        // 4 => B B B C <- cursor, skipped
        // 3 => B B B B
        // 2 => A A A B
        // 1 => A B C D
        //
        // The connectors reverse this to (equivalent to test #2): asc, DESC, DESC, ASC
        // 1 => A B C D
        // 2 => A A A B
        // 3 => B B B B
        // 4 => B B B C <- cursor, skipped
        // 6 => C C D C <- take
        // 5 => C C B A <- take
        //
        // Because the final result (6, 5) gets reversed again to restore original order, the result is:
        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyTestModel(cursor: { id: 4 }, take: -2, skip: 1, orderBy: [{ fieldA: desc }, {fieldB: asc }, {fieldC: asc }, {fieldD: desc }]) {
              id
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    // "A cursor with take, skip and multiple order-bys with the orderBy combination not stable" should "return the expected results"
    #[connector_test(schema(string_combination))]
    async fn cursor_take_skip_multiple_unstable_order(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 1, fieldA: "A", fieldB: "B", fieldC: "C", fieldD: "D"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 2, fieldA: "A", fieldB: "A", fieldC: "A", fieldD: "B"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 3, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 4, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 5, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}){ id }}"#
        );
        run_query!(
            &runner,
            r#"mutation {createOneTestModel(data: { id: 6, fieldA: "C", fieldB: "C", fieldC: "D", fieldD: "C"}){ id }}"#
        );

        // >>> TEST #1
        // Ordered: DESC, ASC, ASC, DESC
        // The order is at the discretion of the db, possible result options:
        // - 3 and 5 are included in the result: (3, 5, 2) | (5, 3, 2)
        // - Only 3 or only 5 are included in the result: (3, 2, 1) | (5, 2, 1)
        // - None of the duplicates is included: (2, 1)
        //
        // One possible query constellation:
        // 6 => C C D C
        // 5 => B B B B
        // 4 => B B B B <- cursor, skipped
        // 3 => B B B B <- take
        // 2 => A A A B <- take
        // 1 => A B C D <- take
        insta::assert_snapshot!(
          run_query!(
            &runner,
            r#"query {
                findManyTestModel(cursor: { id: 4 }, take: 3, skip: 1, orderBy: [{ fieldA: desc }, { fieldB: asc }, { fieldC: asc }, { fieldD: desc }]) {
                  id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":2},{"id":1}]}}"###
        );

        // >>> TEST #2
        // Ordered (reverse from test #1): ASC, DESC, DESC, ASC
        // The order is at the discretion of the db, possible result options (cursor on 4):
        // - 3 and 5 are included in the result: (3, 5, 6) | (5, 3, 6)
        // - Only 3 or only 5 are included in the result: (3, 6) | (5, 6)
        // - None of the duplicates is included: (6)
        //
        // One possible query constellation:
        // 1 => A B C D
        // 2 => A A A B
        // 4 => B B B B <- cursor, skipped
        // 3 => B B B B <- take
        // 5 => B B B B <- take
        // 6 => C C D C <- take
        insta::assert_snapshot!(
          run_query!(
            &runner,
            r#"query {
                findManyTestModel(cursor: { id: 4 }, take: 3, skip: 1, orderBy: [{ fieldA: asc }, { fieldB: desc }, { fieldC: desc }, { fieldD: asc }]) {
                  id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}"###
        );

        // Note: Negative takes reverse the order, the following tests check that.
        // >>> TEST #3, same order as 1, but gets reversed to test 2

        // Originally the query orders: desc, ASC, ASC, DESC (equivalent to test #1).
        // With -3 instead of 3, it wants to take (possibility):
        // 6 => C C D C <- take
        // 5 => B B B B <- take
        // 3 => B B B B <- take
        // 4 => B B B B <- cursor, skipped
        // 2 => A A A B
        // 1 => A B C D
        //
        // The connectors reverse this to (equivalent to test #2): asc, DESC, DESC, ASC
        // 1 => A B C D
        // 2 => A A A B
        // 4 => B B B B <- cursor, skipped
        // 3 => B B B B <- take
        // 5 => B B B B <- take
        // 6 => C C D C <- take
        //
        // Because the final result gets reversed again to restore original order, the result possibilities are the same as #2, just reversed.
        insta::assert_snapshot!(
          run_query!(
            &runner,
            r#"query {
              findManyTestModel(cursor: { id: 4 }, take: -3, skip: 1, orderBy: [{ fieldA: desc }, { fieldB: asc }, { fieldC: asc }, { fieldD: desc }]) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":5}]}}"###
        );

        Ok(())
    }

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
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
