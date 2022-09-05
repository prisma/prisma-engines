use query_engine_tests::*;

/// Reproduction for https://github.com/prisma/prisma/issues/9372
mod create_very_many {
    use super::*;

    fn basic() -> String {
        include_str!("create_very_many.prisma").to_owned()
    }

    #[connector_test(suite = "wat", schema(basic))]
    async fn create_very_many(runner: Runner) -> TestResult<()> {
        use std::fmt::Write;

        const RECORDS: usize = 150_000;

        let mut items = String::with_capacity(3 * RECORDS);
        for i in 0..RECORDS {
            write!(items, "{{term:\"{}\"}},", i).unwrap();
        }

        let query = format!("mutation {{ createOneDictionary(data: {{ entries: {{ create: [{items}] }} }}) {{ id }} }}");
        let result = runner.query(query).await?;
        dbg!(result.to_string_pretty());
        result.assert_success();

        Ok(())
    }
}

/// New test to check that SQL Server doesn't allow writing autoincrement IDs.
#[test_suite(schema(autoinc_id), only(SqlServer))]
mod sql_server {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test]
    async fn disallow_sql_server(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "mutation { createManyTestModel(data: [{ id: 2 }]) { count }}",
            2009,
            "Field does not exist on enclosing type."
        );

        insta::assert_snapshot!(
          run_query!(&runner, "mutation { createManyTestModel(data: [{}]) { count }}"),
          @r###"{"data":{"createManyTestModel":{"count":1}}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(autoinc_id_cockroachdb), only(CockroachDb))]
mod cockroachdb {
    use query_engine_tests::run_query;

    #[connector_test]
    async fn foo(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, "mutation { createManyTestModel(data: [{},{}]) { count }}"),
          @r###"{"data":{"createManyTestModel":{"count":2}}}"###
        );

        let res = run_query_json!(&runner, "query { findManyTestModel { id }}");

        let records = res["data"]["findManyTestModel"].as_array().unwrap();
        assert_eq!(records.len(), 2);
        assert!(records[0]["id"].is_string());
        assert!(records[1]["id"].is_string());

        Ok(())
    }

    #[connector_test]
    async fn foo_sequence(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, "mutation { createManyTestModelSeq(data: [{},{}]) { count }}"),
          @r###"{"data":{"createManyTestModelSeq":{"count":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, "query { findManyTestModelSeq { id }}"),
          @r###"{"data":{"findManyTestModelSeq":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(autoinc_id), capabilities(CreateMany, AutoIncrement))]
mod single_col {
    use query_engine_tests::run_query;

    #[connector_test(exclude(CockroachDb))]
    async fn foo(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, "mutation { createManyTestModel(data: [{},{}]) { count }}"),
          @r###"{"data":{"createManyTestModel":{"count":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, "query { findManyTestModel { id }}"),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }
}
