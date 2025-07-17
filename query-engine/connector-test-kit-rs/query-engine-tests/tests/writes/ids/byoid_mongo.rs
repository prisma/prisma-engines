use query_engine_tests::*;

// "bring_your_own_id_mongo"
#[test_suite(only(MongoDb))]
mod byoi_mongo {
    use indoc::indoc;
    use query_engine_tests::{Runner, assert_error, run_query};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, String, @id, @default(cuid()))
              p        String  @unique
              childOpt Child?  @relation(fields: [childId], references: [id])
              childId  String? @unique
          }

          model Child {
              #id(id, String, @id, @default(cuid()))
              c         String @unique
              parentOpt Parent?
          }"#
        };

        schema.to_owned()
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, String, @id, @default(cuid()))
              p        String @unique
              childOpt Child?
          }

          model Child {
              #id(id, String, @id, @default(cuid()))
              c         String  @unique
              parentOpt Parent? @relation(fields: [parentId],references: [id])
              parentId  String? @unique
          }"#
        };

        schema.to_owned()
    }

    // "A Create Mutation" should "create and return item with own Id"
    #[connector_test(schema(schema_1))]
    async fn create_and_return_item_woi_1(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation {
              createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
            }"#,
            2002,
            "Unique constraint failed on the constraint: `_id_`"
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return item with own Id"
    #[connector_test(schema(schema_2))]
    async fn create_and_return_item_woi_2(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
                createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
              }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation {
                  createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
                }"#,
            2002,
            "Unique constraint failed on the constraint: `_id_`"
        );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid"
    #[connector_test(schema(schema_1))]
    async fn error_for_invalid_id_1_1(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
              createOneParent(data: {p: "Parent", id: 12}){p, id}
            }"#,
            2009,
            "Invalid argument type"
        );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid"
    #[connector_test(schema(schema_2))]
    async fn error_for_invalid_id_1_2(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
                  createOneParent(data: {p: "Parent", id: 12}){p, id}
                }"#,
            2009,
            "Invalid argument type"
        );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid 2"
    #[connector_test(schema(schema_1))]
    async fn error_for_invalid_id_2_1(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
            createOneParent(data: {p: "Parent", id: true}){p, id}
          }"#,
            0,
            "Reason: 'id' String or Int value expected"
        );
        Ok(())
    }

    // "A Nested Create Mutation" should "create and return item with own Id"
    #[connector_test(schema(schema_1))]
    async fn nested_create_return_item_woi_1(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9", childOpt:{create:{c:"Child", id: "5c88f558dee5fb6fe357c7a5"}}}){p, id, childOpt { c, id} }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9","childOpt":{"c":"Child","id":"5c88f558dee5fb6fe357c7a5"}}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation {
              createOneParent(data: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a3", childOpt:{create:{c:"Child 2", id: "5c88f558dee5fb6fe357c7a5"}}}){p, id, childOpt { c, id} }
            }"#,
            2002,
            "Unique constraint failed on the constraint: `_id_`"
        );

        Ok(())
    }

    // "A Nested Create Mutation" should "create and return item with own Id"
    #[connector_test(schema(schema_2))]
    async fn nested_create_return_item_woi_2(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
                createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9", childOpt:{create:{c:"Child", id: "5c88f558dee5fb6fe357c7a5"}}}){p, id, childOpt { c, id} }
              }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9","childOpt":{"c":"Child","id":"5c88f558dee5fb6fe357c7a5"}}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation {
                  createOneParent(data: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a3", childOpt:{create:{c:"Child 2", id: "5c88f558dee5fb6fe357c7a5"}}}){p, id, childOpt { c, id} }
                }"#,
            2002,
            "Unique constraint failed on the constraint: `_id_`"
        );

        Ok(())
    }

    // "An Upsert Mutation" should "work"
    #[connector_test(schema(schema_1))]
    async fn upsert_should_work_1(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
                where: {id: "5c88f558dee5fb6fe357c7a9"}
                create: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a9"}
                update: {p: { set: "Parent 2" }}
                )
              {p, id}
            }"#),
          @r###"{"data":{"upsertOneParent":{"p":"Parent 2","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        Ok(())
    }

    // "An Upsert Mutation" should "work"
    #[connector_test(schema(schema_2))]
    async fn upsert_should_work_2(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
                upsertOneParent(
                    where: {id: "5c88f558dee5fb6fe357c7a9"}
                    create: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a9"}
                    update: {p: { set: "Parent 2" }}
                    )
                  {p, id}
                }"#),
          @r###"{"data":{"upsertOneParent":{"p":"Parent 2","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        Ok(())
    }

    // "An nested Upsert Mutation" should "work"
    #[connector_test(schema(schema_1))]
    async fn nested_upsert_should_work_1(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(
              where: { id: "5c88f558dee5fb6fe357c7a9" }
              data: {
                childOpt: {
                  upsert: {
                    create: { id: "5c88f558dee5fb6fe357c7a4", c: "test 3" }
                    update: { c: { set: "Does not matter" } }
                  }
                }
              }
            ) {
              p
              id
              childOpt {
                c
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9","childOpt":{"c":"test 3","id":"5c88f558dee5fb6fe357c7a4"}}}}"###
        );

        Ok(())
    }

    // "An nested Upsert Mutation" should "work"
    #[connector_test(schema(schema_2))]
    async fn nested_upsert_should_work_2(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(
              where: { id: "5c88f558dee5fb6fe357c7a9" }
              data: {
                childOpt: {
                  upsert: {
                    create: { id: "5c88f558dee5fb6fe357c7a4", c: "test 3" }
                    update: { c: { set: "Does not matter" } }
                  }
                }
              }
            ) {
              p
              id
              childOpt {
                c
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9","childOpt":{"c":"test 3","id":"5c88f558dee5fb6fe357c7a4"}}}}"###
        );

        Ok(())
    }
}
