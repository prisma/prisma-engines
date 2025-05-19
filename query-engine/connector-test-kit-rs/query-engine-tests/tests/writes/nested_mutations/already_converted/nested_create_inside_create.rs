use query_engine_tests::*;

// TODO(dom): All failings except one (only a couple of tests is failing per test)
#[test_suite(exclude(CockroachDb))]
mod create_inside_create {
    use query_engine_tests::{run_query, DatamodelWithParams};
    use query_test_macros::relation_link_test;

    // "a P1 to C1 relation should work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
    async fn p1_c1(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
      }"#),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a PM to C1! should work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1_req(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
                c:"c2"
                c_1: "c2_1"
                c_2: "c2_2"
              }]
            }
          }){
           childrenOpt{
             c
           }
          }
        }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        Ok(())
    }

    // "a P1 to C1! relation  should work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneReq")]
    async fn p1_c1_req(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
        }"#),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    //"a P1 to C1! relation  should work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToOneOpt")]
    async fn pm_c1(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
                  c:"c2"
                  c_1: "c2_1"
                  c_2: "c2_2"
                }]
              }
            }){
             childrenOpt{
               c
             }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        Ok(())
    }

    // "a P1! to CM  relation  should work"
    #[relation_link_test(on_parent = "ToOneReq", on_child = "ToMany")]
    async fn p1_req_cm(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
          }"#),
          @r###"{"data":{"createOneParent":{"childReq":{"c":"c1"}}}}"###
        );

        Ok(())
    }

    // "a P1 to CM relation should work"
    #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToMany")]
    async fn p1_cm(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
          }"#),
          @r###"{"data":{"createOneParent":{"childOpt":{"c":"c1"}}}}"###
        );

        // make sure it is traversable in the opposite direction as well
        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyChild {
              parentsOpt {
                p
              }
            }
          }"#),
          @r###"{"data":{"findManyChild":[{"parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }

    // "a PM to CM relation should work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn pm_cm(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
                  c:"c2"
                  c_1: "c2_1"
                  c_2: "c2_2"
                }]
              }
            }){
             childrenOpt{
               c
             }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        Ok(())
    }
}
