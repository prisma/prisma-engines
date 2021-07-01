use query_engine_tests::*;

// TODO(dom): Not working on mongo.
// All connectors are excluded because it's only supposed to run on MongoDb
// But most of the tests are failing (they are individually marked below anyway)
// Once tests are fixed, change `exclude` to `only(MongoDb)`
#[test_suite(exclude(MongoDb, Postgres, Sqlite, Mysql, SqlServer, Vitess))]
//  bring_your_own_id_mongo
mod byoi_mongo {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, Runner};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, String, @id, @default(cuid()))
              p        String  @unique
              childOpt Child?  @relation(fields: [childId], references: [id])
              childId  String?
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
              parentId  String?
          }"#
        };

        schema.to_owned()
    }

    // "A Create Mutation" should "create and return item with own Id"
    // TODO(dom): Not working on mongo.
    // Wrong error code: got P2002 instad of P3010
    #[connector_test(schema(schema_1))]
    async fn create_and_return_item_woi_1(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
            }"#,
            3010,
            "A unique constraint would be violated on Parent. Details: Field name: id"
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return item with own Id"
    // TODO(dom): Not working on mongo.
    // Wrong error code: got P2002 instad of P3010
    #[connector_test(schema(schema_2))]
    async fn create_and_return_item_woi_2(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
              }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        assert_error!(
            runner,
            r#"mutation {
                  createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
                }"#,
            3010,
            "A unique constraint would be violated on Parent. Details: Field name: id"
        );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid"
    // TODO(dom): Not working on mongo.
    // Wrong error code: got P2009 instad of P3044
    #[connector_test(schema(schema_1))]
    async fn error_for_invalid_id_1_1(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent", id: 12}){p, id}
            }"#,
            3044,
            "You provided an ID that was not a valid MongoObjectId: 12"
        );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid"
    // TODO(dom): Not working on mongo.
    // Wrong error code: got P2009 instad of P3044
    #[connector_test(schema(schema_2))]
    async fn error_for_invalid_id_1_2(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
                  createOneParent(data: {p: "Parent", id: 12}){p, id}
                }"#,
            3044,
            "You provided an ID that was not a valid MongoObjectId: 12"
        );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid 2"
    // TODO(dom): Not working on mongo.
    #[connector_test(schema(schema_1))]
    async fn error_for_invalid_id_2_1(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
            createOneParent(data: {p: "Parent", id: true}){p, id}
          }"#,
            0,
            "Reason: 'id' String or Int value expected"
        );
        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid 2"
    // TODO(dom): Not working on mongo.
    #[connector_test(schema(schema_2))]
    async fn error_for_invalid_id_2_2(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
                createOneParent(data: {p: "Parent", id: true}){p, id}
              }"#,
            0,
            "Reason: 'id' String or Int value expected"
        );
        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid 3"
    // TODO(dom): Works on mongo.
    // Result: {"data":{"createOneParent":{"p":"Parent","id":"this is probably way to long, lets see what error it throws"}}}
    #[connector_test(schema(schema_1))]
    async fn error_for_invalid_id_3_1(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent", id: "this is probably way to long, lets see what error it throws"}){p, id}
            }"#,
            3044,
            "You provided an ID that was not a valid MongoObjectId: this is probably way to long, lets see what error it throws"
        );
        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid 3"
    // TODO(dom): Works on mongo.
    // Result: {"data":{"createOneParent":{"p":"Parent","id":"this is probably way to long, lets see what error it throws"}}}
    #[connector_test(schema(schema_2))]
    async fn error_for_invalid_id_3_2(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent", id: "this is probably way to long, lets see what error it throws"}){p, id}
            }"#,
            3044,
            "You provided an ID that was not a valid MongoObjectId: this is probably way to long, lets see what error it throws"
        );
        Ok(())
    }

    // "A Nested Create Mutation" should "create and return item with own Id"
    // TODO(dom): Not working on mongo.
    // Wrong error code. Got P2002 instead of P3010
    #[connector_test(schema(schema_1))]
    async fn nested_create_return_item_woi_1(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9", childOpt:{create:{c:"Child", id: "5c88f558dee5fb6fe357c7a5"}}}){p, id, childOpt { c, id} }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9","childOpt":{"c":"Child","id":"5c88f558dee5fb6fe357c7a5"}}}}"###
        );

        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a3", childOpt:{create:{c:"Child 2", id: "5c88f558dee5fb6fe357c7a5"}}}){p, id, childOpt { c, id} }
            }"#,
            3010,
            "A unique constraint would be violated on Child. Details: Field name: id"
        );

        Ok(())
    }

    // "A Nested Create Mutation" should "create and return item with own Id"
    // TODO(dom): Not working on mongo.
    // Wrong error code. Got P2002 instead of P3010
    #[connector_test(schema(schema_2))]
    async fn nested_create_return_item_woi_2(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9", childOpt:{create:{c:"Child", id: "5c88f558dee5fb6fe357c7a5"}}}){p, id, childOpt { c, id} }
              }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9","childOpt":{"c":"Child","id":"5c88f558dee5fb6fe357c7a5"}}}}"###
        );

        assert_error!(
            runner,
            r#"mutation {
                  createOneParent(data: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a3", childOpt:{create:{c:"Child 2", id: "5c88f558dee5fb6fe357c7a5"}}}){p, id, childOpt { c, id} }
                }"#,
            3010,
            "A unique constraint would be violated on Child. Details: Field name: id"
        );

        Ok(())
    }

    // "A Nested Create Mutation" should "error with invalid id"
    // TODO(dom): Works on mongo.
    // Result: {"data":{"createOneParent":{"p":"Parent 2","id":"5c88f558dee5fb6fe357c7a9","childOpt":{"c":"Child 2","id":"5c88f558dee5fb6fe357c7a9afafasfsadfasdf"}}}}
    #[connector_test(schema(schema_1))]
    async fn nested_create_error_invalid_id_1(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
            createOneParent(data: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a9", childOpt:{create:{c:"Child 2", id: "5c88f558dee5fb6fe357c7a9afafasfsadfasdf"}}}){p, id, childOpt { c, id} }
          }"#,
            3044,
            "You provided an ID that was not a valid MongoObjectId: 5c88f558dee5fb6fe357c7a9afafasfsadfasdf"
        );

        Ok(())
    }

    // "A Nested Create Mutation" should "error with invalid id"
    // TODO(dom): Works on mongo.
    // Result: {"data":{"createOneParent":{"p":"Parent 2","id":"5c88f558dee5fb6fe357c7a9","childOpt":{"c":"Child 2","id":"5c88f558dee5fb6fe357c7a9afafasfsadfasdf"}}}}
    #[connector_test(schema(schema_2))]
    async fn nested_create_error_invalid_id_2(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
                createOneParent(data: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a9", childOpt:{create:{c:"Child 2", id: "5c88f558dee5fb6fe357c7a9afafasfsadfasdf"}}}){p, id, childOpt { c, id} }
              }"#,
            3044,
            "You provided an ID that was not a valid MongoObjectId: 5c88f558dee5fb6fe357c7a9afafasfsadfasdf"
        );

        Ok(())
    }

    // "An Upsert Mutation" should "work"
    #[connector_test(schema(schema_1))]
    async fn upsert_should_work_1(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
    async fn upsert_should_work_2(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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

    // "An Upsert Mutation" should "error with id that is too long"
    // TODO(dom): Works on mongo.
    // Result: {"data":{"upsertOneParent":{"p":"Parent 2","id":"5c88f558dee5fb6fe357c7a9aggfasffgasdgasg"}}}
    #[connector_test(schema(schema_1))]
    async fn upsert_error_with_too_long_id_1(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              upsertOneParent(
                  where: {id: "5c88f558dee5fb6fe357c7a9"}
                  create: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a9aggfasffgasdgasg"}
                  update: {p: { set: "Parent 2" }}
                  )
                {p, id}
              }"#,
            3044,
            "You provided an ID that was not a valid MongoObjectId: 5c88f558dee5fb6fe357c7a9aggfasffgasdgasg"
        );

        Ok(())
    }

    // "An Upsert Mutation" should "error with id that is too long"
    // TODO(dom): Works on mongo.
    // Result: {"data":{"upsertOneParent":{"p":"Parent 2","id":"5c88f558dee5fb6fe357c7a9aggfasffgasdgasg"}}}
    #[connector_test(schema(schema_2))]
    async fn upsert_error_with_too_long_id_2(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
                  upsertOneParent(
                      where: {id: "5c88f558dee5fb6fe357c7a9"}
                      create: {p: "Parent 2", id: "5c88f558dee5fb6fe357c7a9aggfasffgasdgasg"}
                      update: {p: { set: "Parent 2" }}
                      )
                    {p, id}
                  }"#,
            3044,
            "You provided an ID that was not a valid MongoObjectId: 5c88f558dee5fb6fe357c7a9aggfasffgasdgasg"
        );

        Ok(())
    }

    // "An nested Upsert Mutation" should "work"
    #[connector_test(schema(schema_1))]
    async fn nested_upsert_should_work_1(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
    async fn nested_upsert_should_work_2(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "5c88f558dee5fb6fe357c7a9"}){p, id}
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"5c88f558dee5fb6fe357c7a9"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
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
