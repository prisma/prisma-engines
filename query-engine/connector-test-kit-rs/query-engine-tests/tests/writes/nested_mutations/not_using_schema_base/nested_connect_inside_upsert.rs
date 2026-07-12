use query_engine_tests::*;

#[test_suite(schema(schema))]
mod connect_inside_upsert {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, run_query_json};

    fn schema() -> String {
        let schema = indoc! {
            r#" model Parent {
              #id(id, String, @id, @default(cuid()))
              p        String  @unique
              childOpt Child?  @relation(fields: [childId], references: [id])
              childId  String?
          }

          model Child {
              #id(id, String, @id, @default(cuid()))
              c          String  @unique
              parentsOpt Parent[]
          }"#
        };

        schema.to_owned()
    }

    // "A P1 to CM relation" should "be connectable by id within an upsert in the create case"
    #[connector_test]
    async fn p1_cm_upsert_in_create(runner: Runner) -> TestResult<()> {
        let child_id = run_query_json!(
            &runner,
            r#"mutation { createOneChild(data: {c:"c1"}){ id } }"#,
            &["data", "createOneChild", "id"]
        )
        .to_string();

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation{{upsertOneParent(where: {{id: "5beea4aa6183dd734b2dbd9b"}}, create: {{p: "p1", childOpt:{{connect:{{id:{child_id}}}}}}}, update: {{p: {{ set: "p-new" }}}}) {{
            childOpt{{ c }}
          }}
        }}"#)),
          @r###"{"data":{"upsertOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "A P1 to CM relation" should "be connectable by id within an upsert in the update case"
    #[connector_test]
    async fn p1_cm_upsert_in_update(runner: Runner) -> TestResult<()> {
        let child_id = run_query_json!(
            &runner,
            r#"mutation { createOneChild(data: {c:"c1"}){ id } }"#,
            &["data", "createOneChild", "id"]
        );
        let parent_id = run_query_json!(
            &runner,
            r#"mutation { createOneParent(data: {p:"p1"}){ id } }"#,
            &["data", "createOneParent", "id"]
        );

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation{{upsertOneParent(where: {{id: {parent_id}}}, create: {{p: "p new"}}, update: {{p: {{ set: "p updated" }},childOpt:{{connect:{{id: {child_id}}}}}}}) {{
            childOpt{{c}}
          }}
        }}"#)),
          @r###"{"data":{"upsertOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "A P1 to CM relation" should "be connectable by unique field within an upsert in the update case"
    #[connector_test]
    async fn p1_cm_uniq_upsert_update(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneChild(data: {c:"c1"}){ id } }"#);
        run_query!(&runner, r#"mutation { createOneParent(data: {p:"p1"}){ id } }"#);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
              where: { p: "p1"}
              create: {p: "p new"}
              update: {
                p: { set: "p updated" }
                childOpt:{ connect: {c:"c1" } }
            }) {
            childOpt { c }
          }
        }"#),
          @r###"{"data":{"upsertOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a one to many relation" should "throw the correct error for a connect by unique field within an upsert in the update case"
    #[connector_test]
    async fn one2m_fail_upsert_update(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneChild(data: {c:"c1"}){ id } }"#);
        run_query!(&runner, r#"mutation { createOneParent(data: {p:"p1"}){ id } }"#);

        assert_error!(
            &runner,
            r#"mutation{upsertOneParent(where: {p: "p1"}, create: {p: "new p"}, update: {p: { set: "p updated" },childOpt:{connect:{c:"DOES NOT EXIST"}}}) {
              childOpt{c}
            }
          }"#,
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No 'Child' record (needed to inline the relation on 'Parent' record(s)) was found for a nested connect on one-to-many relation 'ChildToParent'."
        );

        Ok(())
    }
}
