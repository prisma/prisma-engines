use query_engine_tests::*;

#[test_suite]
mod delete_many_rels {
    use indoc::indoc;
    use query_engine_tests::{run_query, Runner};
    use query_test_macros::relation_link_test;

    #[relation_link_test(
        on_parent = "ToOneOpt",
        on_child = "ToOneOpt",
        id_only = true,
        exclude(Sqlite("cfd1"))
    )]
    // "a P1 to C1  relation " should "succeed when trying to delete the parent"
    // On D1, this fails with:
    //
    // ```diff
    // - {"data":{"deleteManyParent":{"count":2}}}
    // + {"data":{"deleteManyParent":{"count":3}}}
    // ```
    async fn p1_c1(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "1"
                  p_2: "2"
                  childOpt: {
                    create: { c: "c1", c_1: "foo", c_2: "bar" }
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
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    // "a P1 to C1  relation " should "succeed when trying to delete the parent if there are no children"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                    p: "p1" p_1: "lol" p_2: "zoop"
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
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    // "a PM to C1!  relation " should "succeed if no child exists that requires the parent"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneReq")]
    async fn pm_c1_req_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                  p: "p1" p_1: "p1" p_2: "p2"
                }){
                  childrenOpt{
                     c
                  }
                }
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    // "a P1 to C1!  relation " should "succeed when trying to delete the parent if there is no child"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                    p: "p1" p_1: "p1" p_2: "p2"
                }){
                  p
                }
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt", exclude(Sqlite("cfd1")))]
    // "a PM to C1 relation " should "succeed in deleting the parent"
    // On D1, this fails with:
    //
    // ```diff
    // - {"data":{"deleteManyParent":{"count":1}}}
    // + {"data":{"deleteManyParent":{"count":3}}}
    // ```
    async fn pm_c1(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "1"
                  p_2: "2"
                  childrenOpt: {
                    create: [{c: "c1", c_1: "foo", c_2: "bar"}, {c: "c2", c_1: "fqe", c_2: "asd"}]
                  }
                }){
                  childrenOpt{
                     c
                  }
                }
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    // "a PM to C1 " should "succeed in deleting the parent if there is no child"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                    p: "p1" p_1: "1" p_2: "2"
                }){
                  p
                }
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    // "a P1! to CM  relation" should "should succeed in deleting the parent "
    #[relation_link_test(on_parent = "ToOneReq", on_child = "ToMany")]
    async fn p1_req_cm_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "1"
                  p_2: "2"
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
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation " should " should succeed in deleting the parent"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_cm(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "1"
                  p_2: "2"
                  childOpt: {
                    create: {c: "c1", c_1: "foo", c_2: "bar"}
                  }
                }){
                  childOpt{
                     c
                  }
                }
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    // "a P1 to CM  relation " should " should succeed in deleting the parent if there is no child"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_cm_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "1"
                  p_2: "2"
                }){
                  p
                }
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany", exclude(Sqlite("cfd1")))]
    // "a PM to CM relation" should "succeed in deleting the parent"
    // On D1, this fails with:
    //
    // ```diff
    // - {"data":{"deleteManyParent":{"count":1}}}
    // + {"data":{"deleteManyParent":{"count":3}}}
    // ```
    async fn pm_cm(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "1"
                  p_2: "2"
                  childrenOpt: {
                    create: [{c: "c1", c_1: "foo", c_2: "bar"},{c: "c2", c_1: "q23", c_2: "lk"}]
                  }
                }){
                  childrenOpt{
                     c
                  }
                }
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    // "a PM to CM  relation" should "succeed in deleting the parent if there is no child"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm_no_children(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation {
                createOneParent(data: {
                  p: "p1"
                  p_1: "1"
                  p_2: "2"
                }){
                  p
                }
              }
        "# })
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    fn additional_schema() -> String {
        let schema = indoc! {
            r#"
            model Parent {
                #id(id, Int, @id)
                #m2m(childrenOpt, Child[], id, Int)

                p            String     @unique
                stepChildId  Int? @unique
                stepChildOpt StepChild? @relation(fields: [stepChildId], references: [id])
            }

            model Child {
                #id(id, Int, @id)
                #m2m(parentsOpt, Parent[], id, Int)

                c          String @unique
            }

            model StepChild {
                #id(id, Int, @id)

                s         String  @unique
                parentOpt Parent?
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(additional_schema), exclude(Sqlite("cfd1")))]
    // "a PM to CM  relation" should "delete the parent from other relations as well"
    // On D1, this fails with:
    //
    // ```diff
    // - {"data":{"deleteManyParent":{"count":1}}}
    // + {"data":{"deleteManyParent":{"count":3}}}
    // ```
    async fn pm_cm_other_relations(runner: Runner) -> TestResult<()> {
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
          run_query!(runner, r#"mutation { deleteManyParent(where: { p: "p1" }) { count }}"#),
          @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }
}
