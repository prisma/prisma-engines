use query_engine_tests::*;

#[test_suite(schema(schema))]
mod prisma_14001 {
    fn schema() -> String {
        r#"
            model TestModel {
                id Int @id @map("_id")
                Field String @unique
            }
        "#
        .to_owned()
    }

    #[connector_test]
    async fn pascal_cased_field_names_work_in_aggregations(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createManyTestModel(data: [{id:1,Field:"two"},{id:2,Field:"three"},{id:3,Field:"one"}]) { count } }"#
        );
        let result = run_query!(
            runner,
            r#"query { findManyTestModel(cursor: {Field: "three"}, orderBy: [{Field: asc}]) { id Field }}"#
        );
        assert_eq!(
            result,
            "{\"data\":{\"findManyTestModel\":[{\"id\":2,\"Field\":\"three\"},{\"id\":1,\"Field\":\"two\"}]}}"
        );
        Ok(())
    }
}
