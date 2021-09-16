use query_engine_tests::*;

// Important: This test covers ALL top level create inputs, like create & upsert,
// because schema building uses the exact same types under the hood.

#[test_suite]
mod unchecked_create {
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

    // "Unchecked creates" should "allow writing inlined relation scalars"
    #[connector_test(schema(schema_1), capabilities(AnyId))]
    async fn allow_writing_inlined_rel_scalars(runner: Runner) -> TestResult<()> {
        // Ensure inserted foreign keys for A are valid.
        run_query!(
            &runner,
            r#"mutation {
          createOneModelB(data: {
            uniq_1: 11
            uniq_2: 12
          }) {
            uniq_1
            uniq_2
          }
        }"#
        );
        run_query!(
            &runner,
            r#"mutation {
          createOneModelC(data: {
            uniq_1: 21
            uniq_2: 22
          }) {
            uniq_1
            uniq_2
          }
        }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: 1
              b_id_1: 11
              b_id_2: 12
              c_id_1: 21
              c_id_2: 22
            }) {
              id
              b {
                uniq_1
                uniq_2
              }
              c {
                uniq_1
                uniq_2
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":1,"b":{"uniq_1":11,"uniq_2":12},"c":{"uniq_1":21,"uniq_2":22}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: 2
              b_id_1: 11
              b_id_2: 12
              c_id_1: null
              c_id_2: 22
            }) {
              id
              b {
                uniq_1
                uniq_2
              }
              c {
                uniq_1
                uniq_2
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":2,"b":{"uniq_1":11,"uniq_2":12},"c":null}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              b_id Int
              c_id Int?

              b ModelB  @relation(fields: [b_id], references: [id])
              c ModelC? @relation(fields: [c_id], references: [id])
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA?
            }

            model ModelC {
              #id(id, Int, @id)
              a  ModelA?
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked creates" should "not allow writing inlined relations regularly"
    #[connector_test(schema(schema_2))]
    async fn disallow_write_inline_rel_regularly(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
              createOneModelA(data: {
                id: 1
                b_id: 11
                c: { create: { id: 21 } }
              }) {
                id
              }
            }"#,
            2009
        );

        Ok(())
    }

    // "Unchecked creates" should "require to write required relation scalars and must allow optionals to be omitted"
    #[connector_test(schema(schema_2))]
    async fn required_write_required_rel_scalars(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
                createOneModelA(data: {
                  id: 1
                }) {
                  id
                }
            }"#,
            2009,
            "`Mutation.createOneModelA.data.ModelAUncheckedCreateInput.b_id`: A value is required but not set."
        );

        run_query!(&runner, r#"mutation { createOneModelB(data: { id: 11 }) { id } }"#);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: 1
              b_id: 11
            }) {
              id
              b { id }
              c { id }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":1,"b":{"id":11},"c":null}}}"###
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              b_id Int
              b ModelB  @relation(fields: [b_id], references: [id])
              c ModelC?
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA?
            }

            model ModelC {
              #id(id, Int, @id)
              a_id Int
              a    ModelA @relation(fields: [a_id], references: [id])
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked creates" should "allow writing non-inlined relations normally"
    #[connector_test(schema(schema_3))]
    async fn allow_write_non_inlined_rel(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneModelB(data: { id: 11 }) { id } }"#);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: 1
              b_id: 11
              c: { create: { id: 21 }}
            }) {
              id
              b { id }
              c { id }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":1,"b":{"id":11},"c":{"id":21}}}}"###
        );

        Ok(())
    }

    fn schema_4() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              b_id Int    @default(11)
              b    ModelB @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA[]
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked creates" should "honor defaults and make required relation scalars optional"
    #[connector_test(schema(schema_4))]
    async fn honor_defaults_make_req_rel_sclrs_opt(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneModelB(data: { id: 11 }) { id } }"#);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: 1
            }) {
              b { id }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"b":{"id":11}}}}"###
        );

        Ok(())
    }

    fn schema_5() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id, @default(autoincrement()))
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked creates" should "allow to write to autoincrement IDs directly"
    #[connector_test(schema(schema_5), capabilities(AutoIncrement, WritableAutoincField))]
    async fn allow_write_autoinc_ids(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: 111
            }) {
              id
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":111}}}"###
        );

        Ok(())
    }
}
