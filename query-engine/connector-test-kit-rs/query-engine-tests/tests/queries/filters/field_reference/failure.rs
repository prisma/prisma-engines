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
            r#"{ findManyTestModel(where: { id: { equals: { ref: "unknown" } } }) { id } }"#,
            2019,
            "The referenced scalar field TestModel.unknown does not exist."
        );

        Ok(())
    }

    #[connector_test]
    async fn relation_field_name_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { id: { equals: { ref: "children" } } }) { id } }"#,
            2019,
            "Expected a referenced scalar field TestModel.children but found a relation field."
        );

        Ok(())
    }

    #[connector_test]
    async fn fields_of_different_type_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { id: { equals: { ref: "str" } } }) { id } }"#,
            2019,
            "Expected a referenced scalar field of type Int but found TestModel.str of type String."
        );

        Ok(())
    }

    #[connector_test(schema(schema_list), capabilities(ScalarLists))]
    async fn field_of_different_arity_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { str: { equals: { ref: "str_list" } } }) { id } }"#,
            2019,
            "error_contains"
        );

        Ok(())
    }

    #[connector_test(schema(schema), exclude(MongoDb, Postgres, CockroachDb))]
    async fn cannot_reference_in_not_in_filter(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { str: { in: { ref: "smth" } } }) { id } }"#,
            2009,
            "Query.findManyTestModel.where.TestModelWhereInput.str.StringFilter.in`: Value types mismatch"
        );

        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { str: { notIn: { ref: "smth" } } }) { id } }"#,
            2009,
            "Query.findManyTestModel.where.TestModelWhereInput.str.StringFilter.notIn`: Value types mismatch"
        );

        Ok(())
    }
}
