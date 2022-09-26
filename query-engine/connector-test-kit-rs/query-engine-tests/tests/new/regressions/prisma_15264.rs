use query_engine_tests::*;

#[test_suite(schema(schema), only(MySql))]
mod prisma_15264 {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {r#"
          model TestModel {
            id    Int    @id @test.UnsignedInt
            email String @unique
          }
        "#};

        schema.to_owned()
    }

    // Regression test for https://github.com/prisma/prisma/issues/15264
    #[connector_test]
    async fn upsert_works_with_unsigned_int(runner: Runner) -> TestResult<()> {
        let query = r#"mutation { upsertOneTestModel(
          where: {
            email: "bob@email.com"
          },
          create: {
            id: 2173158296,
            email: "bob@email.com"
          },
          update: {},
        ) { id email } }"#;

        // create
        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"upsertOneTestModel":{"id":2173158296,"email":"bob@email.com"}}}"###
        );

        // update
        insta::assert_snapshot!(
          run_query!(&runner, query),
          @r###"{"data":{"upsertOneTestModel":{"id":2173158296,"email":"bob@email.com"}}}"###
        );

        Ok(())
    }
}
