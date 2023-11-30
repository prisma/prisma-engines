use query_engine_tests::*;

// "Paging on an 1:m relation with a multi-field orderBy with stable ordering" should "work as expected"
#[test_suite(schema(schema))]
mod paging_one2m_stable_order {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model TestModel {
              #id(id, Int, @id)
              related RelatedTestModel[]
            }

            model RelatedTestModel {
              #id(id, Int, @id)
              fieldA String
              fieldB String
              fieldC String
              fieldD String

              parent_id Int
              parent TestModel @relation(fields: [parent_id], references: [id])

              @@unique([fieldA, fieldB, fieldC, fieldD])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn take_first_child_each_parent(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Ordered: desc, ASC, ASC, DESC
        // 1 => 2 B B B B <- take
        // 1 => 1 A B C D
        // 2 => 3 B A B B <- take
        // 2 => 4 B B B C
        // 3 => 5 C C B A <- take
        // 3 => 6 A C D C
        // Makes: [1 => 2, 2 => 3, 3 => 5]
        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyTestModel(orderBy: { id: asc }) {
              id
              related(take: 1, orderBy: [{ fieldA: desc }, {fieldB: asc }, { fieldC: asc }, { fieldD: desc }]) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"related":[{"id":2}]},{"id":2,"related":[{"id":3}]},{"id":3,"related":[{"id":5}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn take_last_child_each_parent(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Ordered: desc, ASC, ASC, DESC
        // 1 => 2 B B B B
        // 1 => 1 A B C D <- take
        // 2 => 3 B A B B
        // 2 => 4 B B B C <- take
        // 3 => 5 C C B A
        // 3 => 6 A C D C <- take
        // Makes: [1 => 1, 2 => 4, 3 => 6]
        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyTestModel(orderBy: { id: asc}) {
              id
              related(take: -1, orderBy: [{ fieldA: desc }, { fieldB: asc }, { fieldC: asc }, { fieldD: desc }]) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"related":[{"id":1}]},{"id":2,"related":[{"id":4}]},{"id":3,"related":[{"id":6}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_child_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Ordered: desc, ASC, ASC, DESC
        // 1 => 2 B B B B
        // 1 => 1 A B C D
        // 2 => 3 B A B B <- take
        // 2 => 4 B B B C <- take
        // 3 => 5 C C B A
        // 3 => 6 A C D C
        // Makes: [1 => [], 2 => [3, 4], 3 => []]
        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyTestModel {
              id
              related(cursor: { id: 3 }, orderBy: [{ fieldA: desc }, { fieldB: asc }, { fieldC: asc }, { fieldD: desc }]) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"related":[]},{"id":2,"related":[{"id":3},{"id":4}]},{"id":3,"related":[]}]}}"###
        );

        Ok(())
    }

    // Test data:
    // All fields combined are a unique combination (guarantees stable ordering).
    //
    // Parent Child fieldA fieldB fieldC fieldD
    //    1  =>  1      A      B      C      D
    //    1  =>  2      B      B      B      B
    //    2  =>  3      B      A      B      B
    //    2  =>  4      B      B      B      C
    //    3  =>  5      C      C      B      A
    //    3  =>  6      A      C      D      C
    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, related: { create: [{ id: 1, fieldA: "A", fieldB: "B", fieldC: "C", fieldD: "D"}, { id: 2,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}]}}"#).await?;
        create_row(runner, r#"{ id: 2, related: { create: [{ id: 3,  fieldA: "B", fieldB: "A", fieldC: "B", fieldD: "B"},{ id: 4,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "C"}]}}"#).await?;
        create_row(runner, r#"{ id: 3, related: { create: [{ id: 5, fieldA: "C", fieldB: "C", fieldC: "B", fieldD: "A"},{ id: 6,  fieldA: "A", fieldB: "C", fieldC: "D", fieldD: "C"}]}}"#).await?;

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

// "Paging on an 1:m relation with a multi-field orderBy WITHOUT stable ordering" should "work as expected"
#[test_suite(schema(schema))]
mod paging_one2m_unstable_order {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model TestModel {
              #id(id, Int, @id)
              related RelatedTestModel[]
            }

            model RelatedTestModel {
              #id(id, Int, @id)
              fieldA String
              fieldB String
              fieldC String
              fieldD String

              parent_id Int
              parent TestModel @relation(fields: [parent_id], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn take_first_child_each_parent(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Ordered: desc, ASC, ASC, DESC
        // 1 => 2 B B B B <- take
        // 1 => 1 A B C D
        // 2 => 3 B B B B <- take
        // 2 => 4 B B B B <- xor take
        // 3 => 5 C C B A <- take
        // 3 => 6 A C D C
        // Makes: [1 => 2, 2 => 3 | 4, 3 => 5]
        insta::assert_snapshot!(
        run_query!(
          &runner,
          r#"query {
            findManyTestModel(orderBy: { id: asc }) {
              id
              related(take: 1, orderBy: [{ fieldA: desc }, {fieldB: asc }, { fieldC: asc }, { fieldD: desc }]) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"related":[{"id":2}]},{"id":2,"related":[{"id":3}]},{"id":3,"related":[{"id":5}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn take_last_child_each_parent(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Ordered: desc, ASC, ASC, DESC
        // 1 => 2 B B B B
        // 1 => 1 A B C D <- take
        // 2 => 3 B B B B <- take
        // 2 => 4 B B B B <- xor take
        // 3 => 5 C C B A
        // 3 => 6 A C D C <- take
        // Makes: [1 => 1, 2 => 4, 3 => 6]
        insta::assert_snapshot!(
          run_query!(
              &runner,
              r#"query {
                findManyTestModel(orderBy: { id: asc }) {
                  id
                  related(take: -1, orderBy: [{ fieldA: desc }, { fieldB: asc }, { fieldC: asc }, { fieldD: desc }]) {
                    id
                  }
                }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"related":[{"id":1}]},{"id":2,"related":[{"id":3}]},{"id":3,"related":[{"id":6}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn cursor_child_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Ordered: desc, ASC, ASC, DESC
        // 1 => 2 B B B B
        // 1 => 1 A B C D
        // 2 => 3 B B B B <- take
        // 2 => 4 B B B B <- take
        // 3 => 5 C C B A
        // 3 => 6 A C D C
        // Makes: [1 => [], 2 => [3, 4] | [4, 3] | [3] | [4], 3 => []]
        insta::assert_snapshot!(
          run_query!(
            &runner,
            r#"query {
              findManyTestModel {
                id
                related(cursor: { id: 3 }, orderBy: [{ fieldA: desc }, { fieldB: asc }, { fieldC: asc }, { fieldD: desc }]) {
                  id
                }
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"related":[]},{"id":2,"related":[{"id":3},{"id":4}]},{"id":3,"related":[]}]}}"###
        );

        Ok(())
    }

    // Test data:
    // No stable ordering guaranteed.
    //
    // Parent Child fieldA fieldB fieldC fieldD
    //    1  =>  1      A      B      C      D
    //    1  =>  2      B      B      B      B
    //    2  =>  3      B      B      B      B
    //    2  =>  4      B      B      B      B
    //    3  =>  5      C      C      B      A
    //    3  =>  6      A      C      D      C
    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, related: { create: [{ id: 1, fieldA: "A", fieldB: "B", fieldC: "C", fieldD: "D"}, { id: 2,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}]}}"#).await?;
        create_row(runner, r#"{ id: 2, related: { create: [{ id: 3,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"},{ id: 4,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}]}}"#).await?;
        create_row(runner, r#"{ id: 3, related: { create: [{ id: 5, fieldA: "C", fieldB: "C", fieldC: "B", fieldD: "A"},{ id: 6,  fieldA: "A", fieldB: "C", fieldC: "D", fieldD: "C"}]}}"#).await?;

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
