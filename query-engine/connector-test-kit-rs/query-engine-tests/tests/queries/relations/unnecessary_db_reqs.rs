use query_engine_tests::*;

// TODO: Finish porting this suite.
// TODO: Needs a way to count the amount of requests sent for a query
#[test_suite(schema(schema))]
mod unnecessary_db_reqs {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Top {
              #id(id, String, @id)
              middle_id     String?
              middle        Middle? @relation(fields: [middle_id], references: [id])
            }

            model Middle {
              #id(id, String, @id)
              bottom_id     String?
              bottom        Bottom? @relation(fields: [bottom_id], references: [id])
              top           Top[]
            }

            model Bottom {
              #id(id, String, @id)
              bottom        Middle[]
            }"#
        };

        schema.to_owned()
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model Top {
            #id(id, String, @id)
            #m2m(middle, Middle[], String)
          }

          model Middle {
            #id(id, String, @id)
            #m2m(top, Top[], String)
            #m2m(bottom, Bottom[], String)
          }

          model Bottom {
            #id(id, String, @id)
            #m2m(middle, Middle[], String)
          }"#
        };

        schema.to_owned()
    }

    // "One to Many relations" should "not create unnecessary roundtrips"
    #[connector_test(schema(schema_1))]
    async fn one2m_no_roundtrips(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: "lonely_top" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#""#),
          @r###""###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTop(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
