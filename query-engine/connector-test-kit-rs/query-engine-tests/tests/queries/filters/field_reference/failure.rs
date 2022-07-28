use super::setup;

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod failure {
    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              str String
              children Child[]
            }

            model Child {
              #id(id, Int, @id)
              testId Int
              test   TestModel @relation(fields: [testId], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    fn schema_list() -> String {
        let schema = indoc! {
            r#"model TestModel {
            #id(id, Int, @id)
            str      String
            str_list String[]
          }
          "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn unknown_field_name_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { id: { equals: { _ref: "unknown" } } }) { id } }"#,
            2019,
            "The referenced scalar field TestModel.unknown does not exist."
        );

        Ok(())
    }

    #[connector_test]
    async fn relation_field_name_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { id: { equals: { _ref: "children" } } }) { id } }"#,
            2019,
            "Expected a referenced scalar field TestModel.children but found a relation field."
        );

        Ok(())
    }

    #[connector_test]
    async fn fields_of_different_type_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { id: { equals: { _ref: "str" } } }) { id } }"#,
            2019,
            "Expected a referenced scalar field of type Int but found TestModel.str of type String."
        );

        Ok(())
    }

    #[connector_test(schema(schema_list), capabilities(ScalarLists))]
    async fn field_of_different_arity_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { str: { equals: { _ref: "str_list" } } }) { id } }"#,
            2019,
            "Expected a referenced scalar field of type String but found TestModel.str_list of type String[]."
        );

        Ok(())
    }

    // Exclude connectors that supports `ScalarLists`
    #[connector_test(schema(schema), exclude(MongoDb, Postgres, CockroachDb))]
    async fn cannot_reference_in_not_in_filter(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { str: { in: { _ref: "smth" } } }) { id } }"#,
            2009,
            "Query.findManyTestModel.where.TestModelWhereInput.str.StringFilter.in`: Value types mismatch"
        );

        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { str: { notIn: { _ref: "smth" } } }) { id } }"#,
            2009,
            "Query.findManyTestModel.where.TestModelWhereInput.str.StringFilter.notIn`: Value types mismatch"
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_types))]
    async fn ref_field_in_having_must_be_selected(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"query { groupByTestModel(by: [int], having: { int: { _count: { equals: { _ref: "int_2" } } } }) { int }}"#,
            2019,
            ""
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_types))]
    async fn count_requires_int_ref_field(runner: Runner) -> TestResult<()> {
        // assert that referencing a Int field for the count of a string field works
        run_query!(
            &runner,
            r#"query { groupByTestModel(by: [string, int], having: { string: { _count: { equals: { _ref: "int" } } } }) { string, int }}"#
        );

        // assert that the count of a String field expect a the referenced field to be of type Int
        assert_error!(
            runner,
            r#"query { groupByTestModel(by: [string, int], having: { string: { _count: { equals: { _ref: "string" } } } }) { id }}"#,
            2019,
            "Expected a referenced scalar field of type Int but found TestModel.string of type String."
        );

        Ok(())
    }
}
