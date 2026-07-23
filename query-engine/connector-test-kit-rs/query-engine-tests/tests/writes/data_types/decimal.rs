use query_engine_tests::*;

// Ignored for MSSQL and SQLite because of low precision issues.
#[test_suite(schema(schema), capabilities(DecimalType))]
mod decimal {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, Int, @id)
              field Decimal? @default("1.00112233445566778899")
             }"#
        };

        schema.to_owned()
    }

    // TODO(dom): Not working on mongo. Precision issue
    // {"data":{"createOneModel":{"field":"1.00112233445566778899"}}}
    // {"data":{"createOneModel":{"field":"1.001122334455668"}}}
    #[connector_test(exclude(SqlServer, Sqlite, MongoDb))]
    async fn using_decimal_field(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                id: 1
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"createOneModel":{"field":"1.00112233445566778899"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModel(
              where: { id: 1 }
              data: {
                field: "0.09988776655443322"
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"updateOneModel":{"field":"0.09988776655443322"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModel(
              where: { id: 1 }
              data: {
                field: null
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"updateOneModel":{"field":null}}}"###
        );

        Ok(())
    }

    fn deicmal_id() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, Decimal, @id)
             }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(deicmal_id), capabilities(DecimalType), exclude(Sqlserver("mssql.js.wasm")))]
    async fn using_decimal_as_id(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneModel( data: { id: "1000000000" } ) { id } }"#),
          @r###"{"data":{"createOneModel":{"id":"1000000000"}}}"###
        );

        Ok(())
    }
}
