use query_engine_tests::*;

// Important: This test covers ALL nested create inputs, like create nested, connectOrCreate, nested upsert,
// because schema building uses the exact same types under the hood.

#[test_suite]
mod unchecked_nested_create {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              b_id_1 Int
              b_id_2 Int
              c_id_1 Int?
              c_id_2 Int?

              b ModelB  @relation(fields: [b_id_1, b_id_2], references: [uniq_1, uniq_2])
              c ModelC? @relation(fields: [c_id_1, c_id_2], references: [uniq_1, uniq_2])
            }

            model ModelB {
              uniq_1    Int
              uniq_2    Int

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }

            model ModelC {
              uniq_1    Int
              uniq_2    Int

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested creates" should "allow writing non-parent inlined relation scalars"
    // TODO(dom): Not working on mongo (on createOneModelB)
    // {"errors":[{"error":"assertion failed: id_fields.len() == 1","user_facing_error":{"is_panic":true,"message":"assertion failed: id_fields.len() == 1","backtrace":null}}]}
    #[connector_test(schema(schema_1), exclude(MongoDb))]
    async fn allow_write_non_prent_inline_rel_sclrs(runner: &Runner) -> TestResult<()> {
        // B can't be written because it's the parent.
        assert_error!(
            runner,
            r#"mutation {
          createOneModelB(data: {
            uniq_1: 1
            uniq_2: 1
            a: {
              create: {
                id: 1
                b_id_1: 123,
                b_id_2: 321,
              }
            }
          }) {
            uniq_1
            uniq_2
          }
        }"#,
            2009
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelB(data: {
              uniq_1: 2
              uniq_2: 2
              a: {
                create: {
                  id: 2
                }
              }
            }) {
              a {
                b {
                 uniq_1
                 uniq_2
                }
              }
            }
          }"#),
          @r###"{"data":{"createOneModelB":{"a":[{"b":{"uniq_1":2,"uniq_2":2}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelB(data: {
              uniq_1: 3
              uniq_2: 3
              a: {
                create: {
                  id: 3
                  c_id_1: null
                  c_id_2: 123
                }
              }
            }) {
              a {
                b {
                 uniq_1
                 uniq_2
                }
                c {
                  uniq_1
                  uniq_2
                }
              }
            }
          }"#),
          @r###"{"data":{"createOneModelB":{"a":[{"b":{"uniq_1":3,"uniq_2":3},"c":null}]}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              b_id_1 Int
              b_id_2 Int
              c_id_1 Int
              c_id_2 Int

              b ModelB @relation(fields: [b_id_1, b_id_2], references: [uniq_1, uniq_2])
              c ModelC @relation(fields: [c_id_1, c_id_2], references: [uniq_1, uniq_2])
            }

            model ModelB {
              uniq_1    Int
              uniq_2    Int

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }

            model ModelC {
              uniq_1    Int
              uniq_2    Int

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested creates" should "fail if required relation scalars are not provided"
    #[connector_test(schema(schema_2), capabilities(AnyId))]
    async fn fail_if_req_rel_sclr_not_provided(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              createOneModelB(data: {
                uniq_1: 1
                uniq_2: 1
                a: {
                  create: {
                    id: 1
                    c_id_1: 123,
                  }
                }
              }) {
                uniq_1
                uniq_2
              }
            }"#,
            2009,
            "`Mutation.createOneModelB.data.ModelBUncheckedCreateInput.a.ModelAUncheckedCreateNestedManyWithoutBInput.create.ModelAUncheckedCreateWithoutBInput.c_id_2`: A value is required but not set."
        );

        Ok(())
    }

    fn schema_4() -> String {
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

    // "Unchecked nested creates" should "not allow writing inlined relations regularly"
    #[connector_test(schema(schema_4))]
    async fn disallow_writing_inline_rel(runner: &Runner) -> TestResult<()> {
        // We need ModelD to trigger the correct input. We're coming from B, so B is out,
        // then we use C to trigger the union on the unchecked type, then we use d as a regular
        // relation in the input that must fail.
        assert_error!(
            runner,
            r#"mutation {
              createOneModelB(data: {
                id: 1
                a: {
                  create: {
                    id: 1
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

    fn schema_5() -> String {
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

    // "Unchecked nested creates" should "allow writing non-parent, non-inlined relations normally"
    #[connector_test(schema(schema_5))]
    async fn allow_write_non_parent(runner: &Runner) -> TestResult<()> {
        run_query!(runner, r#"mutation { createOneModelD(data: { id: 1 }) { id } }"#);

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelB(data: {
              id: 1
              a: {
                create: {
                  id: 1,
                  d_id: 1
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
          @r###"{"data":{"createOneModelB":{"a":{"c":{"id":1},"d":{"id":1}}}}}"###
        );

        Ok(())
    }

    fn schema_6() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              b_id Int
              c_id Int    @default(1)
              b    ModelB @relation(fields: [b_id], references: [id])
              c    ModelC @relation(fields: [c_id], references: [id])
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA?
            }

            model ModelC {
              #id(id, Int, @id)
              a  ModelA[]
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked nested creates" should "honor defaults and make required relation scalars optional"
    #[connector_test(schema(schema_6))]
    async fn honor_defaults_make_req_rel_sclrs_opt(runner: &Runner) -> TestResult<()> {
        run_query!(runner, r#"mutation { createOneModelC(data: { id: 1 }) { id } }"#);

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelB(data: {
              id: 1
              a: { create: { id: 1 }}
            }) {
              a { c { id }}
            }
          }"#),
          @r###"{"data":{"createOneModelB":{"a":{"c":{"id":1}}}}}"###
        );

        Ok(())
    }

    fn schema_7() -> String {
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

    // "Unchecked nested creates" should "allow to write to autoincrement IDs directly"
    // TODO(dom): Not working on mongo. Expected because no autoincrement() ?
    #[connector_test(schema(schema_7), exclude(SqlServer, MongoDb))]
    async fn allow_write_autoinc_ids(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelB(data: {
              id: 1
              a: { create: { id: 2 }}
            }) {
              a { id }
            }
          }"#),
          @r###"{"data":{"createOneModelB":{"a":{"id":2}}}}"###
        );

        Ok(())
    }
}
