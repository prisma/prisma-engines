//! Regression test for https://github.com/prisma/team-orm/issues/927

use query_engine_tests::*;

#[test_suite(schema(schema))]
mod count_before_relation {
    fn schema() -> String {
        indoc! {
            r#"
            model Parent {
                #id(id, Int, @id)
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parentId Int
                parent   Parent @relation(fields: [parentId], references: [id])
            }
            "#
        }
        .to_owned()
    }

    #[connector_test]
    async fn find_unique(runner: Runner) -> TestResult<()> {
        seed(&runner).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"
                query {
                    findUniqueParent(
                        where: { id: 1 }
                    ) {
                        _count { children }
                        children { id }
                    }
                }
                "#
            ),
            @r###"{"data":{"findUniqueParent":{"_count":{"children":1},"children":[{"id":1}]}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn find_many(runner: Runner) -> TestResult<()> {
        seed(&runner).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"
                query {
                    findManyParent {
                        _count { children }
                        children { id }
                    }
                }
                "#
            ),
            @r###"{"data":{"findManyParent":[{"_count":{"children":1},"children":[{"id":1}]}]}}"###
        );

        Ok(())
    }

    async fn seed(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"
            mutation {
                createOneParent(
                    data: {
                        id: 1,
                        children: {
                            create: { id: 1 }
                        }
                    }
                ) { id }
            }
            "#
        );

        Ok(())
    }
}
