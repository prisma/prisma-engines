use indoc::indoc;
use query_engine_tests::*;

// Important: This test covers ALL nested create inputs, like create nested, connectOrCreate, nested upsert,
// because schema building uses the exact same types under the hood.

#[test_suite]
mod nested_unchecked_update {
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
              uniq_1    String
              uniq_2    String

              a ModelA?

              @@unique([uniq_1, uniq_2])
            }

            model ModelC {
              uniq_1    String
              uniq_2    String

              a ModelA?

              @@unique([uniq_1, uniq_2])
            }"#
        };

        schema.to_owned()
    }

    //"Unchecked nested updates" should "allow writing non-parent inlined relation scalars"
    #[connector_test(schema(schema_1))]
    async fn allow_write_non_prent_inline_rel_sclrs(runner: Runner) -> TestResult<()> {
        // Setup
        run_query!(
            &runner,
            r#"mutation {
                createOneModelA(data: {
                  id: 1
                  b: { create: { uniq_1: "b1_1", uniq_2: "b1_2" }}
                  c: { create: { uniq_1: "c1_1", uniq_2: "c1_2" }}
                }) {
                  id
                }
            }"#
        );
        run_query!(
            &runner,
            r#"mutation {
                createOneModelC(data: {
                  uniq_1: "c2_1"
                  uniq_2: "c2_2"
                }) {
                  uniq_1
                }
            }"#
        );

        // C can be updated for A
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelB(where: {
              uniq_1_uniq_2: {
                uniq_1: "b1_1"
                uniq_2: "b1_2"
              }
            }, data: {
              a: {
                update: {
                  c_id_1: "c2_1"
                  c_id_2: "c2_2"
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
          @r###"{"data":{"updateOneModelB":{"a":{"c":{"uniq_1":"c2_1","uniq_2":"c2_2"}}}}}"###
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
                update: {
                  c_id_1: null
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
          @r###"{"data":{"updateOneModelB":{"a":{"c":null}}}}"###
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
              a  ModelA?
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested updates" should "not allow writing parent inlined relation scalars"
    #[connector_test(schema(schema_2))]
    async fn disallow_write_parent_inline_rel_sclrs(runner: Runner) -> TestResult<()> {
        // B can't be written because it's the parent.

        assert_error!(
            &runner,
            r#"mutation {
              updateOneModelB(where: { id: 1 }, data: {
                a: {
                  update: {
                    b_id: 123
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
              #id(id, Int, @id)
              b_id Int
              c_id Int
              d_id Int

              b ModelB @relation(fields: [b_id], references: [id])
              c ModelC @relation(fields: [c_id], references: [id])
              d ModelD @relation(fields: [d_id], references: [id])
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA?
            }

            model ModelC {
              #id(id, Int, @id)
              a  ModelA?
            }

            model ModelD {
              #id(id, Int, @id)
              a  ModelA?
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested updates" should "not allow writing inlined relations regularly"
    #[connector_test(schema(schema_3))]
    async fn disallow_write_inline_rel(runner: Runner) -> TestResult<()> {
        // We need ModelD to trigger the correct input. We're coming from B, so B is out,
        // then we use C to trigger the union on the unchecked type, then we use d as a regular
        // relation in the input that must fail.
        assert_error!(
            &runner,
            r#"mutation {
                updateOneModelB(data: {
                  a: {
                    update: {
                      c_id: 1
                      d: {
                        create: { id: 1 }
                      }
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

    fn schema_4() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              b_id Int
              d_id Int

              b ModelB  @relation(fields: [b_id], references: [id])
              c ModelC?
              d ModelD  @relation(fields: [d_id], references: [id])
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA?
            }

            model ModelC {
              #id(id, Int, @id)
              a_id Int
              a    ModelA @relation(fields: [a_id], references: [id])
            }

            model ModelD {
              #id(id, Int, @id)
              a  ModelA?
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested updates" should "allow writing non-parent, non-inlined relations normally"
    #[connector_test(schema(schema_4))]
    async fn disallow_write_non_parent(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneModelA(data: {
                  id: 1
                  b: { create: { id: 1 } }
                  d: { create: { id: 1 } }
                }) {
                  id
                }
            }"#
        );
        run_query!(
            &runner,
            r#"mutation {
                createOneModelD(data: {
                  id: 2
                }) {
                  id
                }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelB(where: { id: 1 }, data: {
              a: {
                update: {
                  d_id: 2
                  c: { create: { id: 1 }}
                }
              }
            }) {
              a {
                c { id }
                d { id }
              }
            }
          }"#),
          @r###"{"data":{"updateOneModelB":{"a":{"c":{"id":1},"d":{"id":2}}}}}"###
        );

        Ok(())
    }

    fn schema_5() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id, @default(autoincrement()))
              b_id Int
              b    ModelB @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA?
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested updates" should "allow to write to autoincrement IDs directly"
    #[connector_test(schema(schema_5), exclude(SqlServer))]
    async fn allow_write_autoinc_id(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneModelA(data: { b: { create: { id: 1 }} }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelB(where: { id: 1 }, data: {
              a: { update: { id: 111 }}
            }) {
              a { id }
            }
          }"#),
          @r###"{"data":{"updateOneModelB":{"a":{"id":111}}}}"###
        );

        Ok(())
    }
}
