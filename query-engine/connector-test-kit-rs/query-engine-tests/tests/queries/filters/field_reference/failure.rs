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
              str String

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

            children Child[]
          }

          model Child {
            #id(id, Int, @id)
            str String
            str_list String[]

            testId Int
            test   TestModel @relation(fields: [testId], references: [id])
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
        // Simple scalar filter
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { id: { equals: { _ref: "str" } } }) { id } }"#,
            2019,
            "Expected a referenced scalar field of type Int but found TestModel.str of type String."
        );

        // Through a relation filter
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { children: { some: { id: { equals: { _ref: "str" } } } } }) { id } }"#,
            2019,
            "Expected a referenced scalar field of type Int but found Child.str of type String."
        );

        Ok(())
    }

    #[connector_test(schema(schema_list), capabilities(ScalarLists))]
    async fn field_of_different_arity_fails(runner: Runner) -> TestResult<()> {
        // Simple scalar filter
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { str: { equals: { _ref: "str_list" } } }) { id } }"#,
            2019,
            "Expected a referenced scalar field of type String but found TestModel.str_list of type String[]."
        );

        // Through a relation filter
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { children: { some: { str: { equals: { _ref: "str_list" } } } } }) { id } }"#,
            2019,
            "Expected a referenced scalar field of type String but found Child.str_list of type String[]."
        );

        Ok(())
    }

    // Exclude connectors that supports `ScalarLists`
    // Connectors that don't supports ScalarLists cannot reference fields on inclusion filters
    // since those filters expect scalar lists.
    #[connector_test(schema(schema), exclude(MongoDb, Postgres, CockroachDb))]
    async fn field_ref_inclusion_filter_fails(runner: Runner) -> TestResult<()> {
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
    async fn field_ref_in_having_must_be_selected(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"query { groupByTestModel(by: [int], having: { int: { _count: { equals: { _ref: "int_2" } } } }) { int }}"#,
            2019,
            ""
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_types))]
    async fn count_expect_int_field_ref(runner: Runner) -> TestResult<()> {
        // assert that referencing an Int field for the count of a string field works
        run_query!(
            &runner,
            r#"query { groupByTestModel(by: [string, int], having: { string: { _count: { equals: { _ref: "int" } } } }) { string, int }}"#
        );

        // assert that the count of a String field expect the referenced field to be of type Int
        assert_error!(
            runner,
            r#"query { groupByTestModel(by: [string, int], having: { string: { _count: { equals: { _ref: "string" } } } }) { id }}"#,
            2019,
            "Expected a referenced scalar field of type Int but found TestModel.string of type String."
        );

        Ok(())
    }

    #[connector_test(schema(schemas::json), capabilities(JsonFiltering), exclude(MySql(5.6)))]
    async fn json_string_expect_string_field_ref(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"query { findManyTestModel(where: { json: { string_contains: { _ref: "json" } } }) { id }}"#,
            2019,
            "Expected a referenced scalar field of type String but found TestModel.json of type Json."
        );
        assert_error!(
            runner,
            r#"query { findManyTestModel(where: { NOT: { json: { string_contains: { _ref: "json" } } } }) { id }}"#,
            2019,
            "Expected a referenced scalar field of type String but found TestModel.json of type Json."
        );

        assert_error!(
            runner,
            r#"query { findManyTestModel(where: { json: { string_ends_with: { _ref: "json" } } }) { id }}"#,
            2019,
            "Expected a referenced scalar field of type String but found TestModel.json of type Json."
        );
        assert_error!(
            runner,
            r#"query { findManyTestModel(where: { NOT: { json: { string_ends_with: { _ref: "json" } } } }) { id }}"#,
            2019,
            "Expected a referenced scalar field of type String but found TestModel.json of type Json."
        );

        assert_error!(
            runner,
            r#"query { findManyTestModel(where: { json: { string_starts_with: { _ref: "json" } } }) { id }}"#,
            2019,
            "Expected a referenced scalar field of type String but found TestModel.json of type Json."
        );
        assert_error!(
            runner,
            r#"query { findManyTestModel(where: { NOT: { json: { string_starts_with: { _ref: "json" } } } }) { id }}"#,
            2019,
            "Expected a referenced scalar field of type String but found TestModel.json of type Json."
        );

        Ok(())
    }

    #[connector_test(schema(setup::mixed_composite_types), capabilities(CompositeTypes))]
    async fn referencing_composite_field_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"query { findManyTestModel(where: { comp: { equals: { _ref: "comp" } } }) { id }}"#,
            2009,
            "`Query.findManyTestModel.where.TestModelWhereInput.comp.CompositeNullableCompositeFilter.equals`: Unable to match input value to any allowed input type for the field"
        );

        Ok(())
    }

    /// Json alphanumeric filters don't allow referencing other columns for now because
    /// we can't make it work both for MySQL and MariaDB without making MariaDB its own connector.
    #[connector_test(schema(schemas::json), only(MySql(5.7, 8, "mariadb")))]
    async fn alphanumeric_json_filter_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"query { findManyTestModel(where: { json: { gt: { _ref: "json" } } }) { id }}"#,
            2009,
            "Failed to validate the query: `Value types mismatch."
        );

        Ok(())
    }
}
