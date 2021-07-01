use query_engine_tests::*;

// Important: This test covers ALL top level update inputs, like update & upsert,
// because schema building uses the exact same types under the hood.

#[test_suite]
mod unchecked_update {
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
              uniq_1    String
              uniq_2    String

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }

            model ModelC {
              uniq_1    String
              uniq_2    String

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked updates" should "allow writing inlined relation scalars"
    // TODO(dom): Not working on mongo
    // {"errors":[{"error":"Error occurred during query execution:\nInterpretationError(\"Error for binding \\'0\\'\", Some(QueryGraphBuilderError(RecordNotFound(\"Record to update not found.\"))))","user_facing_error":{"is_panic":false,"message":"An operation failed because it depends on one or more records that were required but not found. Record to update not found.","meta":{"cause":"Record to update not found."},"error_code":"P2025"}}]}
    #[connector_test(schema(schema_1), exclude(MongoDb))]
    async fn allow_write_non_prent_inline_rel_sclrs(runner: &Runner) -> TestResult<()> {
        // Setup
        run_query!(
            runner,
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
            runner,
            r#"mutation {
                createOneModelB(data: {
                  uniq_1: "b2_1"
                  uniq_2: "b2_2"
                }) {
                  uniq_1
                  uniq_2
                }
            }"#
        );
        run_query!(
            runner,
            r#"mutation {
                createOneModelC(data: {
                  uniq_1: "c2_1"
                  uniq_2: "c2_2"
                }) {
                  uniq_1
                  uniq_2
                }
            }"#
        );

        // Update inlined
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneModelA(where: { id: 1 }, data: {
              b_id_1: "b2_1"
              b_id_2: "b2_2"
              c_id_1: "c2_1"
              c_id_2: "c2_2"
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
          @r###"{"data":{"updateOneModelA":{"id":1,"b":{"uniq_1":"b2_1","uniq_2":"b2_2"},"c":{"uniq_1":"c2_1","uniq_2":"c2_2"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneModelA(where: { id: 1 }, data: {
              c_id_1: null
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
          @r###"{"data":{"updateOneModelA":{"id":1,"b":{"uniq_1":"b2_1","uniq_2":"b2_2"},"c":null}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
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

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }

            model ModelC {
              uniq_1    String
              uniq_2    String

              a ModelA[]

              @@unique([uniq_1, uniq_2])
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked updates" should "not allow writing inlined relations regularly"
    #[connector_test(schema(schema_2), capabilities(AnyId))]
    async fn disallow_write_inline_rels(runner: &Runner) -> TestResult<()> {
        // Setup
        run_query!(
            runner,
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

        // Update inlined
        assert_error!(
            runner,
            r#"mutation {
                updateOneModelA(where: { id: 1 }, data: {
                  id: 1
                  b_id_1: "b2_1"
                  b_id_2: "b2_2"
                  c: { create: { uniq_1: "c2_1", uniq_2: "c2_2" } }
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
              b_id Int?
              b    ModelB? @relation(fields: [b_id], references: [id])
              c    ModelC?
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

    // "Unchecked updates" should "allow writing non-inlined relations normally"
    #[connector_test(schema(schema_3))]
    async fn allow_write_non_inline_rels(runner: &Runner) -> TestResult<()> {
        run_query!(runner, r#"mutation { createOneModelB(data: { id: 11 }) { id } }"#);
        run_query!(
            runner,
            r#"mutation {
                createOneModelA(data: {
                  id: 1
                }) {
                  id
                  b { id }
                  c { id }
                }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneModelA(where: { id: 1 }, data: {
              b_id: 11
              c: { create: { id: 21 }}
            }) {
              id
              b { id }
              c { id }
            }
          }"#),
          @r###"{"data":{"updateOneModelA":{"id":1,"b":{"id":11},"c":{"id":21}}}}"###
        );

        Ok(())
    }

    fn schema_4() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id, @default(autoincrement()))
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked updates" should "allow to write to autoincrement IDs directly"
    // TODO(dom): Not working on mongo. Expected because no autoincrement() ?
    #[connector_test(schema(schema_4), exclude(SqlServer, MongoDb))]
    async fn allow_write_autoinc_ids(runner: &Runner) -> TestResult<()> {
        run_query!(runner, r#"mutation { createOneModelA { id } }"#);

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneModelA(where: { id: 1 }, data: { id: 111 }) {
              id
            }
          }"#),
          @r###"{"data":{"updateOneModelA":{"id":111}}}"###
        );

        Ok(())
    }
}
