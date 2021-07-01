use query_engine_tests::*;

// Related issue: https://github.com/prisma/prisma/issues/3081
#[test_suite]
mod graph_reorder {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Company {
              #id(id, String, @id)
              payments Payment[]
            }

            model Visit {
              #id(id, String, @id)
              payment Payment?
            }

            model Payment {
              #id(id, Int, @id)
              company   Company @relation(fields: [companyId], references: [id])
              companyId String
              visit     Visit?  @relation(fields: [visitId], references: [id])
              visitId   String?
            }"#
        };

        schema.to_owned()
    }

    // "The 1:1 relation checks" should "not null out the newly created nested item"
    #[connector_test(schema(schema))]
    async fn test(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneCompany(data:{ id: "company" }) { id } }"#
        );
        run_query!(runner, r#"mutation { createOneVisit(data:{ id:"visit" }) { id } }"#);

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneVisit(
              where: { id: "visit" }
              data: { payment: { create: { id: 1, company: { connect: { id: "company" }}}}}
            ) {
              id
              payment {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneVisit":{"id":"visit","payment":{"id":1}}}}"###
        );

        Ok(())
    }
}
