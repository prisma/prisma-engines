use query_engine_tests::*;

#[test_suite(only(MongoDb))]
mod mongodb {
    use indoc::indoc;

    fn m2m() -> String {
        let schema = indoc! {
            r#"model A {
                id  String  @id @default(dbgenerated()) @map("_id") @test.ObjectId
                gql String?

                b_ids String[]
                bs    B[]      @relation(fields: [b_ids])
            }

            model B {
                id  String  @id @default(dbgenerated()) @map("_id") @test.ObjectId
                gql String?

                a_ids String[] @test.Array(ObjectId)
                as    A[]      @relation(fields: [a_ids])
            }"#
        };

        schema.to_owned()
    }

    /// Makes sure that the m2m relation workaround is explicitly tested.
    #[connector_test(schema(m2m))]
    async fn m2m_syntax_workaround(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(
                &runner,
                r#"mutation {
                    createOneA(
                        data: {
                            id: "507f1f77bcf86cd799439011",
                            bs: {
                                create:[ { id: "507f191e810c19729de860ea" } ]
                            }
                        }
                    ) {
                        id
                        bs { id }
                    }
                }"#
            ),
            @r###"{"data":{"createOneA":{"id":"507f1f77bcf86cd799439011","bs":[{"id":"507f191e810c19729de860ea"}]}}}"###
        );

        Ok(())
    }
}
