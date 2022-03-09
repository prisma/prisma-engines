use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod on_composites {
    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                to_one Composite
            }

            type Composite {
                field String @test.ObjectId
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn native_types_work(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneTestModel(data: { id: 1, to_one: { field: "620fe5542736737e92ea3c36" } }) { id to_one { field }} }"#),
          @r###"{"data":{"createOneTestModel":{"id":1,"to_one":{"field":"620fe5542736737e92ea3c36"}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn invalid_objectid_must_error(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 1, to_one: { field: "nope" } }) { id } }"#,
            2023,
            "Malformed ObjectID"
        );

        Ok(())
    }
}
