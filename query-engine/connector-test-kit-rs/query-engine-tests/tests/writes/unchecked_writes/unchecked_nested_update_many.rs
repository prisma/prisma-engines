use query_engine_tests::*;

#[test_suite]
//  unchecked_nested_updated_many
mod unchecked_nested_um {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)

              b_id_1 String
              b_id_2 String
              c_id_1 String?
              c_id_2 String?

              b ModelB  @relation(fields: [b_id_1, b_id_2], references: [uniq_1, uniq_2])
              c ModelC? @relation(fields: [c_id_1, c_id_2], references: [uniq_1, uniq_2])
            }

            model ModelB {
              #id(id, Int, @id)

              uniq_1    String
              uniq_2    String

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }

            model ModelC {
              #id(id, Int, @id)

              uniq_1    String
              uniq_2    String

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested many updates" should "allow writing non-parent inlined relation scalars"
    #[connector_test(schema(schema_1), exclude(Vitess("planetscale.js")))]
    async fn allow_write_non_prent_inline_rel_sclrs(runner: Runner) -> TestResult<()> {
        // Setup
        // B1 -> A1 -> C1
        // └---> A2 -> C2
        //             C3
        run_query!(
            &runner,
            r#"mutation {
                createOneModelB(data: {
                  id: 1,
                  uniq_1: "b1_1"
                  uniq_2: "b1_2"
                  a: {
                    create: [
                      { id: 1, c: { create: { id: 1, uniq_1: "c1_1", uniq_2: "c1_2" }}},
                      { id: 2, c: { create: { id: 2, uniq_1: "c2_1", uniq_2: "c2_2" }}}
                    ]
                  }
                }) {
                  uniq_1
                }
            }"#
        );

        run_query!(
            &runner,
            r#"mutation {
                createOneModelC(data: {
                  id: 3,
                  uniq_1: "c3_1"
                  uniq_2: "c3_2"
                }) {
                  uniq_1
                }
            }"#
        );

        // Update all As for B1, connecting them to C3
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelB(where: {
              uniq_1_uniq_2: {
                uniq_1: "b1_1"
                uniq_2: "b1_2"
              }
            }, data: {
              a: {
                updateMany: {
                  where: { id: { not: 0 }}
                  data: {
                    c_id_1: "c3_1"
                    c_id_2: "c3_2"
                  }
                }
              }
            }) {
              a {
                c {
                 uniq_1
                 uniq_2
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneModelB":{"a":[{"c":{"uniq_1":"c3_1","uniq_2":"c3_2"}},{"c":{"uniq_1":"c3_1","uniq_2":"c3_2"}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelB(where: {
              uniq_1_uniq_2: {
                uniq_1: "b1_1"
                uniq_2: "b1_2"
              }
            }, data: {
              a: {
                updateMany: {
                  where: { id: { not: 0 }}
                  data: {
                    c_id_1: null
                  }
                }
              }
            }) {
              a {
                c {
                 uniq_1
                 uniq_2
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneModelB":{"a":[{"c":null},{"c":null}]}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              b_id Int
              b    ModelB @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA[]
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested many updates" should "not allow writing parent inlined relation scalars"
    #[connector_test(schema(schema_2))]
    async fn disallow_write_parent_inline_rel_sclrs(runner: Runner) -> TestResult<()> {
        // B can't be written because it's the parent.
        assert_error!(
            &runner,
            r#"mutation {
                updateOneModelB(where: { id: 1 }, data: {
                  a: {
                    updateMany: {
                      where: { id: 1 }
                      data: { b_id: 123 }
                    }
                  }
                }) {
                  id
                }
            }"#,
            2009
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id, @default(autoincrement()))
              b_id Int
              b    ModelB @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA[]
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested many updates" should "allow to write to autoincrement IDs directly"
    #[connector_test(
        schema(schema_3),
        capabilities(AutoIncrement, WritableAutoincField),
        exclude(CockroachDb)
    )]
    async fn allow_write_autoinc_id(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneModelA(data: { b: { create: { id: 1 }} }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelB(where: { id: 1 }, data: {
              a: { updateMany: { where: { id: { not: 0 }}, data: { id: 111 }}}
            }) {
              a { id }
            }
          }"#),
          @r###"{"data":{"updateOneModelB":{"a":[{"id":111}]}}}"###
        );

        Ok(())
    }

    fn schema_3_cockroachdb() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, BigInt, @id, @default(autoincrement()))
              b_id Int
              b    ModelB @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA[]
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested many updates" should "allow to write to autoincrement IDs directly"
    #[connector_test(schema(schema_3_cockroachdb), only(CockroachDb))]
    async fn allow_write_autoinc_id_cockroachdb(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneModelA(data: { b: { create: { id: 1 }} }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelB(where: { id: 1 }, data: {
              a: { updateMany: { where: { id: { not: 0 }}, data: { id: 111 }}}
            }) {
              a { id }
            }
          }"#),
          @r###"{"data":{"updateOneModelB":{"a":[{"id":"111"}]}}}"###
        );

        Ok(())
    }
}
