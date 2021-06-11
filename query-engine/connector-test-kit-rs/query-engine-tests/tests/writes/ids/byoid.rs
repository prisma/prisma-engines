use query_engine_tests::*;

#[test_suite(only(MySql, Postgres, Sqlite, Vitess))]
//  bring_your_own_id
mod byoid {
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
    #[connector_test(schema(schema_1))]
    async fn create_and_return_item_woi_1(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "Own Id"}){p, id}
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"Own Id"}}}"###
        );

        let error_target = match runner.connector() {
            query_engine_tests::ConnectorTag::MySql(_) => "constraint: `PRIMARY`",
            query_engine_tests::ConnectorTag::Vitess(_) => "(not available)",
            _ => "fields: (`id`)",
        };

        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent2", id: "Own Id"}){p, id}
            }"#,
            2002,
            format!("Unique constraint failed on the {}", error_target)
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return item with own Id"
    #[connector_test(schema(schema_2))]
    async fn create_and_return_item_woi_2(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneParent(data: {p: "Parent", id: "Own Id"}){p, id}
              }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"Own Id"}}}"###
        );

        let error_target = match runner.connector() {
            query_engine_tests::ConnectorTag::MySql(_) => "constraint: `PRIMARY`",
            query_engine_tests::ConnectorTag::Vitess(_) => "(not available)",
            _ => "fields: (`id`)",
        };

        assert_error!(
            runner,
            r#"mutation {
                  createOneParent(data: {p: "Parent2", id: "Own Id"}){p, id}
                }"#,
            2002,
            format!("Unique constraint failed on the {}", error_target)
        );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid"
    // FIXME: Does that make sense??
    // #[connector_test(schema(schema_1))]
    // async fn error_for_invalid_id_1_1(runner: &Runner) -> TestResult<()> {
    //     insta::assert_snapshot!(
    //       run_query!(runner, r#"mutation {
    //         createOneParent(data: {p: "Parent", id: 12}){p, id}
    //       }"#),
    //       @r###"{"data":{"createOneParent":{"p":"Parent","id":"12"}}}"###
    //     );

    //     Ok(())
    // }

    // // "A Create Mutation" should "error for id that is invalid"
    // // FIXME: Does that make sense??
    // #[connector_test(schema(schema_2))]
    // async fn error_for_invalid_id_1_2(runner: &Runner) -> TestResult<()> {
    //     insta::assert_snapshot!(
    //       run_query!(runner, r#"mutation {
    //         createOneParent(data: {p: "Parent", id: 12}){p, id}
    //       }"#),
    //       @r###"{"data":{"createOneParent":{"p":"Parent","id":"12"}}}"###
    //     );

    //     Ok(())
    // }

    // "A Create Mutation" should "error for id that is invalid"
    #[connector_test(schema(schema_1))]
    async fn error_for_invalid_id_2_1(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent", id: true}){p, id}
            }"#,
            2009,
            "`Mutation.createOneParent.data.ParentCreateInput.id`: Value types mismatch. Have: Boolean(true), want: String"
        );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid"
    #[connector_test(schema(schema_2))]
    async fn error_for_invalid_id_2_2(runner: &Runner) -> TestResult<()> {
        assert_error!(
                runner,
                r#"mutation {
                  createOneParent(data: {p: "Parent", id: true}){p, id}
                }"#,
                2009,
                "`Mutation.createOneParent.data.ParentCreateInput.id`: Value types mismatch. Have: Boolean(true), want: String"
            );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid"
    // TODO(dom): Actually works on MongoDb.
    // Result: {"data":{"createOneParent":{"p":"Parent","id":"this is probably way to long, lets see what error it throws"}}}
    #[connector_test(schema(schema_1), exclude(Sqlite, Postgres, Mysql, MongoDb, SqlServer))]
    async fn error_for_invalid_id_3_1(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent", id: "this is probably way to long, lets see what error it throws"}){p, id}
            }"#,
            3007,
            "`Mutation.createOneParent.data.ParentCreateInput.id`: Value types mismatch. Have: Boolean(true), want: String"
        );

        Ok(())
    }

    // "A Create Mutation" should "error for id that is invalid"
    // TODO(dom): Actually works on MongoDb.
    // Result: {"data":{"createOneParent":{"p":"Parent","id":"this is probably way to long, lets see what error it throws"}}}
    #[connector_test(schema(schema_2), exclude(Sqlite, Postgres, Mysql, MongoDb, SqlServer))]
    async fn error_for_invalid_id_3_2(runner: &Runner) -> TestResult<()> {
        assert_error!(
                runner,
                r#"mutation {
                  createOneParent(data: {p: "Parent", id: "this is probably way to long, lets see what error it throws"}){p, id}
                }"#,
                3007,
                "`Mutation.createOneParent.data.ParentCreateInput.id`: Value types mismatch. Have: Boolean(true), want: String"
            );

        Ok(())
    }

    // "A Nested Create Mutation" should "create and return item with own Id"
    #[connector_test(schema(schema_1))]
    async fn nested_create_return_item_woi_1(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "Own Id", childOpt:{create:{c:"Child", id: "Own Child Id"}}}){p, id, childOpt { c, id} }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"Own Id","childOpt":{"c":"Child","id":"Own Child Id"}}}}"###
        );

        let error_target = match runner.connector() {
            query_engine_tests::ConnectorTag::MySql(_) => "constraint: `PRIMARY`",
            query_engine_tests::ConnectorTag::Vitess(_) => "(not available)",
            _ => "fields: (`id`)",
        };

        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent 2", id: "Own Id 2", childOpt:{create:{c:"Child 2", id: "Own Child Id"}}}){p, id, childOpt { c, id} }
            }"#,
            2002,
            format!("Unique constraint failed on the {}", error_target)
        );

        Ok(())
    }

    // "A Nested Create Mutation" should "create and return item with own Id"
    #[connector_test(schema(schema_2))]
    async fn nested_create_return_item_woi_2(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneParent(data: {p: "Parent", id: "Own Id", childOpt:{create:{c:"Child", id: "Own Child Id"}}}){p, id, childOpt { c, id} }
              }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"Own Id","childOpt":{"c":"Child","id":"Own Child Id"}}}}"###
        );

        let error_target = match runner.connector() {
            query_engine_tests::ConnectorTag::MySql(_) => "constraint: `PRIMARY`",
            query_engine_tests::ConnectorTag::Vitess(_) => "(not available)",
            _ => "fields: (`id`)",
        };

        assert_error!(
            runner,
            r#"mutation {
                  createOneParent(data: {p: "Parent 2", id: "Own Id 2", childOpt:{create:{c:"Child 2", id: "Own Child Id"}}}){p, id, childOpt { c, id} }
                }"#,
            2002,
            format!("Unique constraint failed on the {}", error_target)
        );

        Ok(())
    }

    // "A Nested Create Mutation" should "error with invalid id"
    // TODO: Should we really validate this
    // TODO(dom): Actually works on MongoDb.
    // Result: {"data":{"createOneParent":{"p":"Parent","id":"this is probably way to long, lets see what error it throws"}}}
    #[connector_test(schema(schema_1), exclude(Sqlite, Postgres, Mysql, MongoDb, SqlServer))]
    async fn nested_create_invalid_id_1(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent 2", id: "Own Id 2", childOpt:{create:{c:"Child 2", id: "This is way too long and should error"}}}){p, id, childOpt { c, id} }
            }"#,
            3007,
            "Value for field id is too long."
        );

        Ok(())
    }

    // "A Nested Create Mutation" should "error with invalid id"
    // TODO: Should we really validate this
    // TODO(dom): Actually works on MongoDb.
    // Result: {"data":{"createOneParent":{"p":"Parent","id":"this is probably way to long, lets see what error it throws"}}}
    #[connector_test(schema(schema_2), exclude(Sqlite, Postgres, Mysql, MongoDb, SqlServer))]
    async fn nested_create_invalid_id_2(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              createOneParent(data: {p: "Parent 2", id: "Own Id 2", childOpt:{create:{c:"Child 2", id: "This is way too long and should error"}}}){p, id, childOpt { c, id} }
            }"#,
            3007,
            "Value for field id is too long."
        );

        Ok(())
    }

    // "An Upsert Mutation" should "work"
    #[connector_test(schema(schema_1))]
    async fn upsert_should_work_1(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            upsertOneParent(
                where: {id: "Does not exist"}
                create: {p: "Parent 2", id: "Own Id"}
                update: {p: { set: "Parent 2"} }
                )
              {p, id}
            }"#),
          @r###"{"data":{"upsertOneParent":{"p":"Parent 2","id":"Own Id"}}}"###
        );

        Ok(())
    }

    // "An Upsert Mutation" should "work"
    #[connector_test(schema(schema_2))]
    async fn upsert_should_work_2(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                upsertOneParent(
                    where: {id: "Does not exist"}
                    create: {p: "Parent 2", id: "Own Id"}
                    update: {p: { set: "Parent 2"} }
                    )
                  {p, id}
                }"#),
          @r###"{"data":{"upsertOneParent":{"p":"Parent 2","id":"Own Id"}}}"###
        );

        Ok(())
    }

    // "An Upsert Mutation" should "error with id that is too long"
    // TODO(dom): Actually works on MongoDb.
    // Result: {"data":{"upsertOneParent":{"p":"Parent 2","id":"Way way too long for a proper id"}}}
    #[connector_test(schema(schema_1), exclude(Sqlite, Postgres, Mysql, MongoDb, SqlServer))]
    async fn upsert_error_with_id_too_long_1(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
              upsertOneParent(
                  where: {id: "Does not exist"}
                  create: {p: "Parent 2", id: "Way way too long for a proper id"}
                  update: {p: { set: "Parent 2" }}
                  )
                {p, id}
              }"#,
            3007,
            "Value for field id is too long."
        );

        Ok(())
    }

    // "An Upsert Mutation" should "error with id that is too long"
    // TODO(dom): Actually works on MongoDb.
    // Result: {"data":{"upsertOneParent":{"p":"Parent 2","id":"Way way too long for a proper id"}}}
    #[connector_test(schema(schema_2), exclude(Sqlite, Postgres, Mysql, MongoDb, SqlServer))]
    async fn upsert_error_with_id_too_long_2(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
                  upsertOneParent(
                      where: {id: "Does not exist"}
                      create: {p: "Parent 2", id: "Way way too long for a proper id"}
                      update: {p: { set: "Parent 2" }}
                      )
                    {p, id}
                  }"#,
            3007,
            "Value for field id is too long."
        );

        Ok(())
    }

    // "An nested Upsert Mutation" should "work"
    #[connector_test(schema(schema_1))]
    async fn nested_upsert_should_work_1(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: {p: "Parent", id: "Own Id"}){p, id}
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"Own Id"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneParent(
                where: {id: "Own Id"}
                data: {
                    childOpt: {upsert:{
                          create:{ id: "Own Id 3", c: "test 3"}
                          update:{ c: { set: "Does not matter" } }
                    }}
                  }
                )
              {p, id, childOpt{c, id}}
            }"#),
          @r###"{"data":{"updateOneParent":{"p":"Parent","id":"Own Id","childOpt":{"c":"test 3","id":"Own Id 3"}}}}"###
        );

        Ok(())
    }

    // "An nested Upsert Mutation" should "work"
    #[connector_test(schema(schema_2))]
    async fn nested_upsert_should_work_2(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneParent(data: {p: "Parent", id: "Own Id"}){p, id}
              }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent","id":"Own Id"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                updateOneParent(
                    where: {id: "Own Id"}
                    data: {
                        childOpt: {upsert:{
                              create:{ id: "Own Id 3", c: "test 3"}
                              update:{ c: { set: "Does not matter" } }
                        }}
                      }
                    )
                  {p, id, childOpt{c, id}}
                }"#),
          @r###"{"data":{"updateOneParent":{"p":"Parent","id":"Own Id","childOpt":{"c":"test 3","id":"Own Id 3"}}}}"###
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model Blog {
              #id(myId, String, @id, @default(cuid()))
              name String
            }"#
        };

        schema.to_owned()
    }

    // "An id field with a custom name" should "work"
    #[connector_test(schema(schema_3))]
    async fn id_field_custom_name_should_work(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneBlog(data: {name: "MyBlog"}){ name }
          }"#),
          @r###"{"data":{"createOneBlog":{"name":"MyBlog"}}}"###
        );

        Ok(())
    }
}
