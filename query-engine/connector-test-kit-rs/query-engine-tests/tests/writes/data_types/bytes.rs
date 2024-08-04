use query_engine_tests::*;

#[test_suite]
mod bytes {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn basic() -> String {
        let schema = indoc! {
            r#"
              model Model {
                #id(id, Int, @id)
                field Bytes? @default("dGVzdA==")
              }
            "#
        };

        schema.to_owned()
    }

    fn bytes_id() -> String {
        let schema = indoc! {
            r#"
            model BytesId {
              #id(id, Bytes, @id)
            }
          "#
        };

        schema.to_owned()
    }

    // "Using a bytes field" should "work"
    #[connector_test(schema(basic))]
    async fn using_bytes_field(runner: Runner) -> TestResult<()> {
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
          @r###"{"data":{"createOneModel":{"field":"dGVzdA=="}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModel(
              where: { id: 1 }
              data: {
                field: "dA=="
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"updateOneModel":{"field":"dA=="}}}"###
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

    #[connector_test(schema(bytes_id), exclude(MySQL, Vitess, SqlServer,))]
    async fn byte_id_coercion(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"
            mutation { createOneBytesId(data: { id: "dGVzdA==" }) { id } }
          "#),
          @r###"{"data":{"createOneBytesId":{"id":"dGVzdA=="}}}"###
        );

        Ok(())
    }
}
