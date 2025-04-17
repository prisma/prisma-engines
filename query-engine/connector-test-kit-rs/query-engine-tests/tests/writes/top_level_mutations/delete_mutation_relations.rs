use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
mod delete_mutation_relations {
    use indoc::indoc;
    use query_engine_tests::{run_query, Runner};
    use query_test_macros::relation_link_test;

    // "a P1 to C1  relation " should "succeed when trying to delete the parent"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt", id_only = true)]
    async fn p1_c1(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
            mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "p_1"
                  p_2: "p_2"
                  childOpt: {
                    create: {
                      c: "c1"
                      c_1: "c_1"
                      c_2: "c_2"
                    }
                  }
                }){
                  p
                  childOpt{
                     c
                  }
                }
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a P1 to C1  relation " should "succeed when trying to delete the parent"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_no_children(runner: &Runner, t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(format!(
                r#" mutation {{
                    createOneParent(data: {{
                        p: "p1"
                        p_1: "p_1"
                        p_2: "p_2"
                    }}) {{
                        {}
                    }}
                }}
            "#,
                t.parent().selection()
            ))
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a PM to C1!  relation " should "succeed if no child exists that requires the parent"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "p_1"
                  p_2: "p_2"
                }){
                  childrenOpt{
                     c
                  }
                }
              }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a P1 to C1!  relation " should "succeed when trying to delete the parent if there is no child"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      p: "p1"
                      p_1: "p_1"
                      p_2: "p_2"
                    }){
                      p
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a PM to C1 " should "succeed in deleting the parent"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      p: "p1"
                      p_1: "p_1"
                      p_2: "p_2"
                      childrenOpt: {
                        create: [{
                          c: "c1"
                          c_1: "c_1"
                          c_2: "c_2"
                        }, {
                          c: "c2"
                          c_1: "c2_1"
                          c_2: "p2_2"
                        }]
                      }
                    }){
                      childrenOpt{
                         c
                      }
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a PM to C1 " should "succeed in deleting the parent if there is no child"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      p: "p1"
                      p_1: "p_1"
                      p_2: "p_2"
                    }){
                      p
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a P1! to CM  relation" should "should succeed in deleting the parent"
    #[relation_link_test(on_parent = "ToOneReq", on_child = "ToMany")]
    async fn p1_req_cm(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      p: "p1"
                      p_1: "p_1"
                      p_2: "p_2"
                      childReq: {
                        create: {
                          c: "c1"
                          c_1: "c_1"
                          c_2: "c_2"
                        }
                      }
                    }){
                      childReq{
                         c
                      }
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation " should " should succeed in deleting the parent"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_cm(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      p: "p1"
                      p_1: "p_1"
                      p_2: "p_2"
                      childOpt: {
                        create: {
                          c: "c1"
                          c_1: "c_1"
                          c_2: "c_2"
                        }
                      }
                    }){
                      childOpt{
                         c
                      }
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a P1 to CM relation " should " should succeed in deleting the parent if there is no child"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_cm_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      p: "p1"
                      p_1: "p_1"
                      p_2: "p_2"
                    }){
                      p
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation" should "succeed in deleting the parent"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      p: "p1"
                      p_1: "p_1"
                      p_2: "p_2"
                      childrenOpt: {
                        create: [{
                          c: "c1"
                          c_1: "c_1"
                          c_2: "c_2"
                        },{
                          c: "c2"
                          c_1: "c2_1"
                          c_2: "c2_2"
                        }]
                      }
                    }){
                      childrenOpt{
                         c
                      }
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation" should "succeed in deleting the parent if there is no child"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      p: "p1"
                      p_1: "p_1"
                      p_2: "p_2"
                    }){
                      p
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    fn additional_schema_1() -> String {
        let schema = indoc! {
            r#"
            model Parent {
                #id(id, Int, @id)
                #m2m(childrenOpt, Child[], id, Int)

                p            String     @unique
                stepChildOpt StepChild?
            }

               model Child {
                #id(id, Int, @id)
                #m2m(parentsOpt, Parent[], id, Int)

                c          String @unique
               }

               model StepChild {
                #id(id, Int, @id)

                s         String  @unique
                parentId  Int? @unique
                parentOpt Parent? @relation(fields: [parentId], references: [id])
               }"#
        };

        schema.to_owned()
    }

    fn additional_schema_2() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                #m2m(childrenOpt, Child[], id, Int)

                p            String     @unique
                stepChildId  Int?       @unique
                stepChildOpt StepChild? @relation(fields: [stepChildId], references: [id])
               }

               model Child {
                #id(id, Int, @id)
                #m2m(parentsOpt, Parent[], id, Int)

                c          String   @unique
               }

               model StepChild {
                #id(id, Int, @id)

                s         String  @unique
                parentOpt Parent?
               }"#
        };

        schema.to_owned()
    }

    // "a PM to CM  relation" should "delete the parent from other relations as well"
    #[connector_test(schema(additional_schema_1))]
    async fn pm_cm_other_relations_1(runner: Runner) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      id: 1,
                      p: "p1"
                      childrenOpt: {
                        create: [{id: 1, c: "c1"},{id: 2, c: "c2"}]
                      }
                      stepChildOpt: {
                        create: {id: 1, s: "s1"}
                      }
                    }){
                      p
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation" should "delete the parent from other relations as well"
    #[connector_test(schema(additional_schema_2))]
    async fn pm_cm_other_relations_2(runner: Runner) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneParent(data: {
                      id: 1,
                      p: "p1"
                      childrenOpt: {
                        create: [{id: 1, c: "c1"},{id: 2, c: "c2"}]
                      }
                      stepChildOpt: {
                        create: {id: 1, s: "s1"}
                      }
                    }){
                      p
                    }
                  }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { p:"p1" }) { p }}"#),
          @r###"{"data":{"deleteOneParent":{"p":"p1"}}}"###
        );

        Ok(())
    }
}
