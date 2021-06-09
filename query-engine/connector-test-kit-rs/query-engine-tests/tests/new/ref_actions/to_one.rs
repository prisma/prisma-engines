use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod to_one {
    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Parent {
                id            String    @id @map("_id")
                p             String    @unique
                p_1           String
                p_2           String
                childOpt      Child?
                non_unique    String?

                @@unique([p_1, p_2])
            }

            model Child {
                id            String    @id @default(cuid()) @map("_id")
                c             String    @unique
                c_1           String
                c_2           String
                parentOpt     Parent?   @relation(fields: [parentRef], references: [p])
                parentRef     String?
                non_unique    String?

                @@unique([c_1, c_2])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn vanilla(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {  createOneParent(data: { id: "1", p: "p1", p_1: "p", p_2: "1" childOpt: { create: {c: "c1", c_1: "foo", c_2: "bar"}    }  }){    p    childOpt{       c    }  }}"#),
          @r###"{"data":{"createOneParent":{"p":"p1","childOpt":{"c":"c1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {  upsertOneParent(  where: { p: "p1" }  update:{    p: { set: "p2" }    childOpt: {delete: true}  }  create:{id: "whatever" ,p: "Should not matter", p_1: "no", p_2: "yes"}  ){    childOpt {      c    }  }}"#),
          @r###"{"data":{"upsertOneParent":{"childOpt":null}}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ uniqueField: 1, nonUniqFieldA: "A", nonUniqFieldB: "A"}"#).await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?;
        Ok(())
    }
}
