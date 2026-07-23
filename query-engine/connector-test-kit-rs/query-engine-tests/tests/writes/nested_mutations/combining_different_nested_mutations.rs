use query_engine_tests::*;

#[test_suite(exclude(CockroachDb))]
mod many_nested_muts {
    use query_engine_tests::{DatamodelWithParams, run_query};
    use query_test_macros::relation_link_test;

    //hardcoded execution order
    //  nestedCreates
    //  nestedUpdates
    //  nestedUpserts
    //  nestedDeletes
    //  nestedConnects
    //  nestedSets
    //  nestedDisconnects
    //  nestedUpdateManys
    //  nestedDeleteManys
    // this could be extended to more combinations and to different schemata
    // the error behavior would be interesting to test, which error is returned, does rollback work
    // "A create followed by an update" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn create_then_update(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneParent(
            data: { p: "p1", p_1: "1", p_2: "2" childrenOpt: { create: [{ c: "c1", c_1: "foo", c_2: "bar" }, { c: "c2", c_1: "q1t", c_2: "asd" }] } }
          ) {
            childrenOpt(orderBy: { c: asc }) {
              c
            }
          }
        }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          updateOneParent(
            where: { p: "p1" }
            data: {
              childrenOpt: {
                create: [{ c: "c3", c_1: "jeesus", c_2: "maria" }, { c: "c4", c_1: "3t1", c_2: "a1" }]
                update: [{ where: { c: "c3" }, data: { c: { set: "cUpdated" } } }]
              }
            }
          ) {
            childrenOpt(orderBy: { c: asc }) {
              c
            }
          }
        }"#),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c4"},{"c":"cUpdated"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild(orderBy: { c: asc }){c, parentsOpt(orderBy: { p: asc }){p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c4","parentsOpt":[{"p":"p1"}]},{"c":"cUpdated","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }

    // "A create followed by a delete" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn create_then_delete(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(
              data: { p: "p1", p_1: "1", p_2: "2" childrenOpt: { create: [{ c: "c1", c_1: "foo", c_2: "bar" }, { c: "c2", c_1: "q1t", c_2: "asd" }] } }
            ) {
              childrenOpt(orderBy: { c: asc }) {
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneParent(
              where: { p: "p1" }
              data: {
                childrenOpt: {
                  create: [{ c: "c3", c_1: "jeesus", c_2: "maria" }, { c: "c4", c_1: "3t1", c_2: "a1" }]
                  update: [{ where: { c: "c3" }, data: { c: { set: "cUpdated" } } }]
                }
              }
            ) {
              childrenOpt(orderBy: { c: asc }) {
                c
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c4"},{"c":"cUpdated"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild(orderBy: { c: asc }){c, parentsOpt(orderBy: { p: asc }){p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c4","parentsOpt":[{"p":"p1"}]},{"c":"cUpdated","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }

    // "A create followed by a set" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn create_then_set(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(
              data: { p: "p1", p_1: "1", p_2: "2", childrenOpt: { create: [{ c: "c1", c_1: "foo", c_2: "bar" }, { c: "c2", c_1: "om", c_2: "mo" }] } }
            ) {
              childrenOpt {
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneParent(
              where: { p: "p1" }
              data: {
                childrenOpt: { create: [{ c: "c3", c_1: "yksi", c_2: "kaksi" }, { c: "c4", c_1: "kolme", c_2: "neljae" }], set: [{ c: "c3" }] }
              }
            ) {
              childrenOpt {
                c
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c3"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[]},{"c":"c2","parentsOpt":[]},{"c":"c3","parentsOpt":[{"p":"p1"}]},{"c":"c4","parentsOpt":[]}]}}"###
        );

        Ok(())
    }

    // "A create followed by an upsert" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn create_then_upsert(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(
              data: { p: "p1", p_1: "1", p_2: "2", childrenOpt: { create: [{ c: "c1", c_1: "1", c_2: "2" }, { c: "c2", c_1: "3", c_2: "4" }] } }
            ) {
              childrenOpt(orderBy: { c: asc }) {
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneParent(
              where: { p: "p1" }
              data: {
                childrenOpt: {
                  create: [{ c: "c3", c_1: "5", c_2: "6" }, { c: "c4", c_1: "7", c_2: "8" }]
                  upsert: [
                    {
                      where: { c: "c3" }
                      create: { c: "should not matter", c_1: "no matter", c_2: "matter not" }
                      update: { c: { set: "cUpdated" }}
                    }
                    {
                      where: { c: "c5" }
                      create: { c: "cNew", c_1: "matter", c_2: "most" }
                      update: { c: { set: "should not matter" }}
                    }
                  ]
                }
              }
            ) {
              childrenOpt(orderBy: { c: asc }) {
                c
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c4"},{"c":"cNew"},{"c":"cUpdated"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild(orderBy: { c: asc }){c, parentsOpt(orderBy: { p: asc }){p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c4","parentsOpt":[{"p":"p1"}]},{"c":"cNew","parentsOpt":[{"p":"p1"}]},{"c":"cUpdated","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }

    // "A create followed by a disconnect" should "work"
    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn create_then_disconnect(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(
              data: { p: "p1", p_1: "1", p_2: "2", childrenOpt: { create: [{ c: "c1", c_1: "foo", c_2: "bar" }, { c: "c2", c_1: "asd", c_2: "qawf" }] } }
            ) {
              childrenOpt {
                c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneParent(
              where: { p: "p1" }
              data: {
                childrenOpt: {
                  create: [{ c: "c3", c_1: "yksi", c_2: "kaksi" }, { c: "c4", c_1: "kolme", c_2: "neljae" }]
                  disconnect: [{ c: "c3" }]
                }
              }
            ) {
              childrenOpt {
                c
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c4"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyChild{c, parentsOpt{p}}}"#),
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c3","parentsOpt":[]},{"c":"c4","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }
}
