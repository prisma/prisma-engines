use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(OrderByNullsFirstLast))]
mod order_by_nulls {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                uniq   Int @unique
                name   String?
                age    Int?
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn simple_nulls_first(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: { name: { sort: asc, nulls: first } }) {
              id
              name
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3,"name":null},{"id":4,"name":null},{"id":1,"name":"A"},{"id":2,"name":"B"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: { name: { sort: desc, nulls: first } }) {
              id
              name
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3,"name":null},{"id":4,"name":null},{"id":2,"name":"B"},{"id":1,"name":"A"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn simple_nulls_last(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: { name: { sort: asc, nulls: last } }) {
              id
              name
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"name":"A"},{"id":2,"name":"B"},{"id":3,"name":null},{"id":4,"name":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: { name: { sort: desc, nulls: last } }) {
              id
              name
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2,"name":"B"},{"id":1,"name":"A"},{"id":3,"name":null},{"id":4,"name":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn two_fields_nulls_last(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: [
              { name: { sort: asc, nulls: last } },
              { age: { sort: asc, nulls: last } },
            ]) {
              name
              age
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"name":"A","age":1},{"name":"B","age":null},{"name":null,"age":2},{"name":null,"age":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: [
              { name: { sort: desc, nulls: last } },
              { age: { sort: asc, nulls: last } },
            ]) {
              name
              age
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"name":"B","age":null},{"name":"A","age":1},{"name":null,"age":2},{"name":null,"age":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: [
              { name: { sort: desc, nulls: last } },
              { age: { sort: desc, nulls: last } },
            ]) {
              name
              age
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"name":"B","age":null},{"name":"A","age":1},{"name":null,"age":2},{"name":null,"age":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: [
              { name: { sort: asc, nulls: last } },
              { age: { sort: desc, nulls: last } },
            ]) {
              name
              age
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"name":"A","age":1},{"name":"B","age":null},{"name":null,"age":2},{"name":null,"age":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn two_fields_nulls_first(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: [
              { name: { sort: asc, nulls: first } },
              { age: { sort: asc, nulls: first } },
            ]) {
              name
              age
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"name":null,"age":null},{"name":null,"age":2},{"name":"A","age":1},{"name":"B","age":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: [
              { name: { sort: desc, nulls: first } },
              { age: { sort: asc, nulls: first } },
            ]) {
              name
              age
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"name":null,"age":null},{"name":null,"age":2},{"name":"B","age":null},{"name":"A","age":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: [
              { name: { sort: desc, nulls: first } },
              { age: { sort: desc, nulls: first } },
            ]) {
              name
              age
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"name":null,"age":null},{"name":null,"age":2},{"name":"B","age":null},{"name":"A","age":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(orderBy: [
              { name: { sort: asc, nulls: first } },
              { age: { sort: desc, nulls: first } },
            ]) {
              name
              age
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"name":null,"age":null},{"name":null,"age":2},{"name":"A","age":1},{"name":"B","age":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn nulls_first_cursor(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // ------ASC NULLS FIRST ORDERINGS------

        // | uniq | name |
        // |------|------|
        // |    3 |      |
        // |    4 |      |
        // |    1 | A    | <== cursor
        // |    2 | B    |
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 1 },
            orderBy: { name: { sort: asc, nulls: first } }
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":1,"name":"A"},{"uniq":2,"name":"B"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 1 },
            take: 3,
            skip: 1,
            orderBy: { name: { sort: asc, nulls: first } }
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":2,"name":"B"}]}}"###
        );

        // reverse: (2, 1, 4, 3)
        // cursor: (1, 4, 3)
        // take: (1, 4)
        // reverse: (4, 1)
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 1 },
            take: -2,
            orderBy: [{ name: { sort: asc, nulls: first } }, { id: asc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":4,"name":null},{"uniq":1,"name":"A"}]}}"###
        );

        // reverse: (2, 1, 4, 3)
        // cursor: (1, 4, 3)
        // skip: (4, 3)
        // take: (4, 3)
        // reverse: (3, 4)
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 1 },
            take: -2,
            skip: 1,
            orderBy: [{ name: { sort: asc, nulls: first } }, { id: asc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":3,"name":null},{"uniq":4,"name":null}]}}"###
        );

        // ------DESC NULLS FIRST ORDERINGS------

        // | uniq | name |
        // |------|------|
        // |    4 |      |
        // |    3 |      | <== cursor
        // |    2 | B    |
        // |    1 | A    |
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 3 },
            orderBy: [{ name: { sort: desc, nulls: first } }, { id: desc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":3,"name":null},{"uniq":2,"name":"B"},{"uniq":1,"name":"A"}]}}"###
        );

        // reverse: (1, 2, 3, 4)
        // cursor: (3, 4)
        // take: (3, 4)
        // reverse: (4, 3)
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 3 },
            take: -2,
            orderBy: [{ name: { sort: desc, nulls: first } }, { id: desc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":4,"name":null},{"uniq":3,"name":null}]}}"###
        );

        // reverse: (1, 2, 3, 4)
        // cursor: (3, 4)
        // skip: (4)
        // take: (4)
        // reverse: (4)
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 3 },
            take: -2,
            skip: 1,
            orderBy: [{ name: { sort: desc, nulls: first } }, { id: desc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":4,"name":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn nulls_last_cursor(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // ------ASC NULLS LAST ORDERINGS------

        // | uniq | name |
        // |------|------|
        // |    1 | A    |
        // |    2 | B    | <== cursor
        // |    3 |      |
        // |    4 |      |
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 2 },
            orderBy: { name: { sort: asc, nulls: last } }
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":2,"name":"B"},{"uniq":3,"name":null},{"uniq":4,"name":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 2 },
            take: 3,
            skip: 1,
            orderBy: { name: { sort: asc, nulls: last } }
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":3,"name":null},{"uniq":4,"name":null}]}}"###
        );

        // reverse: (4, 3, 2, 1)
        // cursor: (2, 1)
        // take: (2, 1)
        // reverse: (1, 2)
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 2 },
            take: -2,
            orderBy: [{ name: { sort: asc, nulls: last } }, { id: asc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":1,"name":"A"},{"uniq":2,"name":"B"}]}}"###
        );

        // reverse: (4, 3, 2, 1)
        // cursor: (2, 1)
        // skip: (1)
        // take: (1)
        // reverse: (1)
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 2 },
            take: -2,
            skip: 1,
            orderBy: [{ name: { sort: asc, nulls: last } }, { id: asc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":1,"name":"A"}]}}"###
        );

        // ------DESC NULLS FIRST ORDERINGS------

        // | uniq | name |
        // |------|------|
        // |    2 | B    |
        // |    1 | A    | <== cursor
        // |    4 |      |
        // |    3 |      |
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 1 },
            orderBy: [{ name: { sort: desc, nulls: last } }, { id: desc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":1,"name":"A"},{"uniq":4,"name":null},{"uniq":3,"name":null}]}}"###
        );

        // reverse: (3, 4, 1, 2)
        // cursor: (1, 2)
        // take: (1, 2)
        // reverse: (2, 1)
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 1 },
            take: -2,
            orderBy: [{ name: { sort: desc, nulls: last } }, { id: desc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":2,"name":"B"},{"uniq":1,"name":"A"}]}}"###
        );

        // reverse: (3, 4, 1, 2)
        // cursor: (1, 2)
        // skip: (2)
        // take: (2)
        // reverse: (2)
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            cursor: { uniq: 1 },
            take: -2,
            skip: 1,
            orderBy: [{ name: { sort: desc, nulls: last } }, { id: desc }]
          ) { uniq name } }"#),
          @r###"{"data":{"findManyTestModel":[{"uniq":2,"name":"B"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn nulls_on_required_field_should_fail(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(orderBy: { id: { sort: asc, nulls: first } }) { id } }"#,
            2009,
            "Value types mismatch"
        );

        Ok(())
    }

    #[connector_test(schema(common_list_types), capabilities(ScalarLists, OrderByNullsFirstLast))]
    async fn nulls_on_list_field_should_fail(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(orderBy: { string: { sort: asc, nulls: first } }) { id } }"#,
            2009,
            "Value types mismatch"
        );

        Ok(())
    }

    #[connector_test]
    async fn ordering_by_nulls_should_be_optional(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(orderBy: [{ id: asc }, { name: { sort: asc } }]) { name } }"#),
          @r###"{"data":{"findManyTestModel":[{"name":"A"},{"name":"B"},{"name":null},{"name":null}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, uniq: 1, name: "A", age: 1}"#).await?;
        create_row(runner, r#"{ id: 2, uniq: 2, name: "B" }"#).await?;
        create_row(runner, r#"{ id: 3, uniq: 3, age: 2}"#).await?;
        create_row(runner, r#"{ id: 4, uniq: 4 }"#).await?;

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
