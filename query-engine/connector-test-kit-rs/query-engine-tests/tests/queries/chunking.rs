use query_engine_tests::*;

/// * QUERY_BATCH_SIZE for testing is 10, configured in direnv.
/// * It should be called QUERY_CHUNK_SIZE instead, because it's a knob to configure query chunking
///  which is splitting queries with more arguments than accepted by the database, in multiple
///  queries.
/// * WASM versions of the engine don't allow for runtime configuration of this value so they default
///  the mininum supported by any database on a SQL family (eg. Postgres, MySQL, SQLite, SQL Server,
///  etc.) As such, in order to guarantee chunking happens, a large number of arguments --larger
///  than the default-- needs to be used, to have actual coverage of chunking code while exercising
///  WASM query engines.
#[test_suite(schema(schema))]
mod chunking {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model A {
                #id(id, Int, @id)
                b_id Int
                c_id Int
                text String

                b B @relation(fields: [b_id], references: [id])
                c C @relation(fields: [c_id], references: [id])
              }

              model B {
                #id(id, Int, @id)
                as A[]
              }

              model C {
                #id(id, Int, @id)
                as A[]
              }
            "#
        };

        schema.to_owned()
    }

    // "chunking of IN queries" should "work when having more than the specified amount of items"
    // TODO(joins): Excluded because we have no support for chunked queries with joins. In practice, it should happen under much less circumstances
    // TODO(joins): than with the query-based strategy, because we don't issue `WHERE IN (parent_ids)` queries anymore to resolve relations.
    #[connector_test(exclude_features("relationJoins"))]
    async fn in_more_items(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyA(where: { id: { in: [5,4,3,2,1,1,1,2,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }}) { id }
            }"# }),
          @r###"{"data":{"findManyA":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    // "ascending ordering of chunked IN queries" should "work when having more than the specified amount of items"
    // TODO(joins): Excluded because we have no support for chunked queries with joins. In practice, it should happen under much less circumstances
    // TODO(joins): than with the query-based strategy, because we don't issue `WHERE IN (parent_ids)` queries anymore to resolve relations.
    #[connector_test(exclude_features("relationJoins"))]
    async fn asc_in_ordering(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyA(where: { id: { in: [5,4,3,2,1,2,1,1,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }}, orderBy: { id: asc }) { id }
            }"# }),
          @r###"{"data":{"findManyA":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    // "ascending ordering of chunked IN queries" should "work when having more than the specified amount of items"
    // TODO(joins): Excluded because we have no support for chunked queries with joins. In practice, it should happen under much less circumstances
    // TODO(joins): than with the query-based strategy, because we don't issue `WHERE IN (parent_ids)` queries anymore to resolve relations.
    #[connector_test(exclude_features("relationJoins"))]
    async fn desc_in_ordering(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyA(where: {id: { in: [5,4,3,2,1,1,1,2,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }}, orderBy: { id: desc }) { id }
            }"# }),
          @r###"{"data":{"findManyA":[{"id":5},{"id":4},{"id":3},{"id":2},{"id":1}]}}"###
        );

        Ok(())
    }

    #[connector_test(exclude(MongoDb, Sqlite("cfd1")))]
    async fn order_by_aggregation_should_fail(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            with_id_excess!(&runner, "query { findManyA(where: {id: { in: [:id_list:] }}, orderBy: { b: { as: { _count: asc } } } ) { id } }"),
            2029 // QueryParameterLimitExceeded
        );

        Ok(())
    }

    #[connector_test(capabilities(FullTextSearchWithoutIndex), exclude(MongoDb))]
    async fn order_by_relevance_should_fail(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            with_id_excess!(
                &runner,
                r#"query { findManyA(where: {id: { in: [:id_list:] }}, orderBy: { _relevance: { fields: text, search: "something", sort: asc } } ) { id } }"#
            ),
            2029 // QueryParameterLimitExceeded
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_a(
            runner,
            r#"{ id: 1, text: "", b: { create: { id: 1 }} c: { create: { id: 1 }} }"#,
        )
        .await?;
        create_a(
            runner,
            r#"{ id: 2, text: "", b: { connect: { id: 1 }} c: { create: { id: 2 }} }"#,
        )
        .await?;
        create_a(
            runner,
            r#"{ id: 3, text: "", b: { create: { id: 3 }} c: { create: { id: 3 }} }"#,
        )
        .await?;
        create_a(
            runner,
            r#"{ id: 4, text: "", b: { create: { id: 4 }} c: { create: { id: 4 }} }"#,
        )
        .await?;
        create_a(
            runner,
            r#"{ id: 5, text: "", b: { create: { id: 5 }} c: { create: { id: 5 }} }"#,
        )
        .await?;

        Ok(())
    }

    async fn create_a(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneA(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();

        Ok(())
    }
}
