use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod prisma_18186 {
    fn schema() -> String {
        let schema = indoc! {
        r#"model A {
            id   Int    @id
            name String

            b B? @relation("a_to_b")
            }

            model B {
            id   Int
            name String

            a_id Int? @unique
            a    A?   @relation("a_to_b", fields: [a_id], references: [id])

            @@unique([id, name])
            }
         "#
        };

        schema.to_owned()
    }

    // Excluded on MongoDB because all models require an @id attribute
    // Excluded on SQLServer because models with unique nulls can't have multiple NULLs, unlike other dbs.
    #[connector_test(exclude(MongoDb, SqlServer))]
    async fn regression(runner: Runner) -> TestResult<()> {
        for i in 0..2 {
            run_query!(
                &runner,
                format!(
                    r#"mutation {{
                        createOneA(data: {{ id: {}, name: "a {}" }}) {{
                            id
                        }}
                    }}"#,
                    i, i
                )
            );

            run_query!(
                &runner,
                format!(
                    r#"mutation {{
                        createOneB(data: {{ id: {}, name: "b {}" }}) {{
                            id
                        }}
                    }}"#,
                    i, i
                )
            );
        }

        insta::assert_snapshot!(run_query!(
            &runner,
            r#"mutation {
                updateOneA(where: {id: 0}, data: { b: { connect: { id_name: { id: 1, name: "b 1"}}}}) {
                    id
                    b {
                        name
                    }   
                }
            }"#),
            @r###"{"data":{"updateOneA":{"id":0,"b":{"name":"b 1"}}}}"###
        );

        Ok(())
    }
}
