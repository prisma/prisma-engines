use query_engine_tests::*;

#[test_suite(schema(schema), exclude(SqlServer))]
// non_embedded_setting_node_selector_to_null
mod non_embedded_node_sel_to_null {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
              b    String? @unique
              key  String  @unique
              c_id Int?

              c C? @relation(fields: [c_id], references: [id])
            }

            model C {
              #id(id, Int, @id)
              c  String?
              a  A[]
            }"#
        };

        schema.to_owned()
    }

    // "Setting a where value to null " should "should only update one if there are several nulls for the specified node selector"
    #[connector_test]
    async fn where_val_to_null(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, b: "abc", key: "abc", c: { create: { id: 1, c: "C" } } }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 2, b: null, key: "abc2", c: { create: { id: 2, c: "C2" } } }"#,
        )
        .await?;

        run_query!(
            &runner,
            r#"mutation {
          updateOneA(
            where: { b: "abc" }
            data: { b: { set: null }, c: { update: { c: { set: "NewC" } } } }
          ) {
            b
            c {
              c
            }
          }
        }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyA(orderBy: { id: asc }) {
              b
              c {
                c
              }
            }
          }"#),
          @r###"{"data":{"findManyA":[{"b":null,"c":{"c":"NewC"}},{"b":null,"c":{"c":"C2"}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneA(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
