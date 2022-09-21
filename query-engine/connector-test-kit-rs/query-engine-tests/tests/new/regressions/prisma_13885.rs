//! https://github.com/prisma/prisma/issues/14447

use query_engine_tests::*;

#[test_suite(schema(schema))]
mod prisma_13885 {
    use indoc::indoc;

    fn schema() -> String {
        let s = indoc! {r#"
            model A {
              id  Int  @id @default(autoincrement())
              bId Int? @unique
              b   B?
            }

            model B {
              id Int @id @default(autoincrement())
              a  A   @relation(fields: [id], references: [bId])
            }
        "#};

        s.into()
    }

    #[connector_test]
    async fn a_special_one_on_one_works(runner: Runner) -> TestResult<()> {
        let query = indoc! {r#"
            mutation {
              createOneA(data: {}) { id }
            }
        "#};

        run_query!(&runner, query);

        let query = indoc! {r#"
            mutation {
              createOneB(data: {
                a: { connect: { id: 1 } }
              }) {
                id
                a {
                  id
                }
              }
            }
        "#};

        let result = run_query_pretty!(&runner, query);

        insta::assert_snapshot!(result, @r###"
        "###);

        Ok(())
    }
}
