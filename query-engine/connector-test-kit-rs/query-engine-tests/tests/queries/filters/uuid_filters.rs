use query_engine_tests::*;

#[test_suite(schema(schema), only(Postgres))]
mod uuid_filter_spec {
    fn schema() -> String {
        r#"
            model Cat {
                id String @test.Uuid @id
            }
        "#
        .to_owned()
    }

    // LIKE queries, or in general substring queries on postgres do not work on UUID columns. Our API
    // must reflect that.
    #[connector_test]
    async fn contains_filter_is_rejected(runner: Runner) -> TestResult<()> {
        // fb37a902-f54b-4520-8c90-8c0e4c7fd31d
        // 5c517461-3eb8-4fd9-85ae-83582d2ee003
        // execute the insert
        run_query!(
            runner,
            r#"mutation { createManyCat(data: [{ id: "fb37a902-f54b-4520-8c90-8c0e4c7fd31d" }, { id: "5c517461-3eb8-4fd9-85ae-83582d2ee003" } ]) { count } }"#
        );

        // check that equality works
        let filtered = run_query!(
            runner,
            r#"query { findManyCat(where: { id: "fb37a902-f54b-4520-8c90-8c0e4c7fd31d" }) { id } }"#
        );
        assert_eq!(
            filtered,
            "{\"data\":{\"findManyCat\":[{\"id\":\"fb37a902-f54b-4520-8c90-8c0e4c7fd31d\"}]}}"
        );

        // check that contains does not
        assert_error!(
            runner,
            r#"query { findManyCat(where: { id: { contains: "fb37a902-f54b-4520-8c90-8c0e4c7fd31d" } }) { id } }"#,
            2009,
            r#"Query.findManyCat.where.CatWhereInput.id.UuidFilter.contains`: Field does not exist on enclosing type."#
        );

        Ok(())
    }
}
