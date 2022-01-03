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

    fn oid_list() -> String {
        let schema = indoc! {
            r#"
            model A {
                id  String  @id @default(dbgenerated()) @map("_id") @test.ObjectId
                list_field String[] @test.Array(ObjectId)
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(oid_list))]
    async fn objectid_list_operations(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(
                &runner,
                r#"mutation {
                    createOneA(
                        data: {
                            id: "507f1f77bcf86cd799439011",
                            list_field: ["507f191e810c19729de860ea", "507f191e810c19729de860ea"]
                        }
                    ) {
                        id
                        list_field
                    }
                }"#
            ),
            @r###"{"data":{"createOneA":{"id":"507f1f77bcf86cd799439011","list_field":["507f191e810c19729de860ea","507f191e810c19729de860ea"]}}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                &runner,
                r#"mutation {
                    updateOneA(
                        where: { id: "507f1f77bcf86cd799439011" }
                        data: {
                            list_field: { set: ["507f191e810c19729de860ea"] }
                        }
                    ) {
                        id
                        list_field
                    }
                }"#
            ),
            @r###"{"data":{"updateOneA":{"id":"507f1f77bcf86cd799439011","list_field":["507f191e810c19729de860ea"]}}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                &runner,
                r#"mutation {
                    updateOneA(
                        where: { id: "507f1f77bcf86cd799439011" }
                        data: {
                            list_field: { push: "61cd963c5488078418a9f125" }
                        }
                    ) {
                        id
                        list_field
                    }
                }"#
            ),
            @r###"{"data":{"updateOneA":{"id":"507f1f77bcf86cd799439011","list_field":["507f191e810c19729de860ea","61cd963c5488078418a9f125"]}}}"###
        );

        // Check that array syntax also works.
        insta::assert_snapshot!(
            run_query!(
                &runner,
                r#"mutation {
                    updateOneA(
                        where: { id: "507f1f77bcf86cd799439011" }
                        data: {
                            list_field: { push: ["61cd96565488078418a9f126", "61cd96605488078418a9f127"] }
                        }
                    ) {
                        id
                        list_field
                    }
                }"#
            ),
            @r###"{"data":{"updateOneA":{"id":"507f1f77bcf86cd799439011","list_field":["507f191e810c19729de860ea","61cd963c5488078418a9f125","61cd96565488078418a9f126","61cd96605488078418a9f127"]}}}"###
        );

        // push 2

        Ok(())
    }
}
