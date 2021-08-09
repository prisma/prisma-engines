use query_engine_tests::*;

#[test_suite(capabilities(CompoundIds))]
mod named_compound_uniques {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_w_named_compounds() -> String {
        let schema = indoc! {
            r#"model Parent {
              name     String
              age      Int

              @@id([name, age], name: "CompoundId")
            }

            model Child {
              name     String
              age      Int

              @@unique([name, age], name: "CompoundUnique")
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_w_named_compounds))]
    async fn using_named_compounds_works(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: { name: "Leto" , age: 50}) {
              name
              age
            }
          }"#),
          @r###"{"data":{"createOneParent":{"name":"Leto","age":50}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneChild(data: { name: "Paul" , age: 20}) {
              name
              age
            }
          }"#),
          @r###"{"data":{"createOneChild":{"name":"Paul","age":20}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneParent(
              where: {
                CompoundId: {
                  name: "Leto"
                  age: 50
                }
              }
              data: { age: { set: 51 }}
            ) {
              name
              age
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"name":"Leto","age":51}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneChild(
              where: {
                CompoundUnique: {
                  name: "Paul"
                  age: 20
                }
              }
              data: { age: { set: 21 }}
            ) {
              name
              age
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"name":"Paul","age":21}}}"###
        );

        Ok(())
    }
}
