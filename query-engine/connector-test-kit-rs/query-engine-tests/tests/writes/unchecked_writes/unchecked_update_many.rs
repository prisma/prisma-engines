use indoc::indoc;
use query_engine_tests::*;

#[test_suite]
mod unchecked_update_many {
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

    // "Unchecked update many" should "allow writing inlined relation scalars"
    #[connector_test(schema(schema_1))]
    async fn allow_write_non_prent_inline_rel_sclrs(runner: &Runner) -> TestResult<()> {
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
                createOneModelA(data: {
                  id: 2
                  b: { create: { uniq_1: "b2_1", uniq_2: "b2_2" }}
                  c: { create: { uniq_1: "c2_1", uniq_2: "c2_2" }}
                }) {
                  id
                }
            }"#
        );

        // Connect all As to b2 and c2
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateManyModelA(where: { id: { not: 0 } }, data: {
              b_id_1: "b2_1"
              b_id_2: "b2_2"
              c_id_1: "c2_1"
              c_id_2: "c2_2"
            }) {
              count
            }
          }"#),
          @r###"{"data":{"updateManyModelA":{"count":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateManyModelA(where: { id: { not: 0 }}, data: {
              c_id_1: null
            }) {
              count
            }
          }"#),
          @r###"{"data":{"updateManyModelA":{"count":2}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              int Int @default(autoincrement())

              @@index([int])
            }"#
        };

        schema.to_owned()
    }

    // "Unchecked updates" should "allow to write to autoincrement IDs directly"
    #[connector_test(schema(schema_2), exclude(SqlServer, Sqlite))]
    async fn allow_write_autoinc_id(runner: &Runner) -> TestResult<()> {
        run_query!(runner, r#"mutation { createOneModelA(data: { id: 1 }) { id } }"#);
        run_query!(runner, r#"mutation { createOneModelA(data: { id: 2 }) { id } }"#);

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateManyModelA(where: { id: { not: 0 }}, data: { int: 111 }) {
              count
            }
          }"#),
          @r###"{"data":{"updateManyModelA":{"count":2}}}"###
        );

        Ok(())
    }
}
