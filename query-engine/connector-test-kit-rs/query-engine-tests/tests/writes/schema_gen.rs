use query_engine_tests::*;
#[test_suite]
mod schema_gen {
    use query_engine_tests::run_query;
    use query_test_macros::relation_link_test;

    #[relation_link_test(on_parent = "ToMany", on_child = "ToMany")]
    async fn m2m(runner: &Runner, _t: &DatamodelWithParams) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneParent(
            data: { p: "p1", p_1: "1", p_2: "2", childrenOpt: { create: [{ c: "c1", c_1: "foo", c_2: "bar" }, { c: "c2", c_1: "zol", c_2: "lol" }] } }
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
              childrenOpt: { create: [{ c: "c3", c_1: "yksi", c_2: "kaksi" }, { c: "c4", c_1: "kolme", c_2: "nelja" }], delete: [{ c: "c3" }] }
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
          @r###"{"data":{"findManyChild":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c4","parentsOpt":[{"p":"p1"}]}]}}"###
        );

        Ok(())
    }
}
