use query_engine_tests::*;

// TODO: Finish porting this test suite. Needs a way to port the `schemaWithRelation` method
#[test_suite]
mod delete_many_rels {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Parent{
              #id(id, Int, @id)
              p  String @unique
              c  Child[]
          }
          
          model Child{
              #id(id, Int, @id)
              c         String @unique
              parentId  Int
              parentReq Parent @relation(fields: [parentId], references: [id])
          }"#
        };

        schema.to_owned()
    }

    // "a PM to C1! relation " should "error when deleting the parent"
    #[connector_test(schema(schema_1))]
    async fn pm_c1_error_delete_parent(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
        createOneChild(data: {
          id: 1,
          c: "c1"
          parentReq: {
            create: {id: 1, p: "p1"}
          }
        }){
          id
        }
      }"#
        );

        assert_error!(
          runner,
          r#"mutation {
            deleteManyParent(
              where: { p: { equals: "p1" }}
            ){
              count
            }
          }"#,
          2014,
          "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    // "a PM to C1! relation " should "error when deleting the parent with empty filter"
    #[connector_test(schema(schema_1))]
    async fn pm_c1_error_delete_parent_empty_filter(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
          createOneChild(data: {
            id: 1,
            c: "c1"
            parentReq: {
              create: {id: 1, p: "p1"}
            }
          }){
            id
          }
        }"#
        );

        assert_error!(
          runner,
          r#"mutation {
            deleteManyParent(where: {}){
              count
            }
          }"#,
          2014,
          "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }
}
