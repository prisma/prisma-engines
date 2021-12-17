use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDB, Vitess))]
mod emulate_ref_integrity {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              uniq_1     Int
              uniq_2     Int
              comments Comment[]
              post     Post?
            
              @@unique([uniq_1, uniq_2])
            }
            
            model Post {
              #id(id, Int, @id)
              name       String?
              authorId_1 Int
              authorId_2 Int
              author     User      @relation(fields: [authorId_1, authorId_2], references: [uniq_1, uniq_2], onUpdate: Cascade, onDelete: Cascade)
              comment    Comment[]
            }
            
            model Comment {
              #id(id, Int, @id)
              name          String?
              writtenById_1 Int
              writtenById_2 Int
              writtenBy     User?   @relation(fields: [writtenById_1, writtenById_2], references: [uniq_1, uniq_2], onUpdate: Cascade, onDelete: Cascade)
              post          Post    @relation(fields: [writtenById_1], references: [id], onUpdate: Cascade, onDelete: Cascade)
            }"#
        };

        schema.to_owned()
    }

    fn nested() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              uniq     Int       @unique

              comments Comment[]
              post     Post?
            }
            
            model Post {
              #id(id, Int, @id)
              authorId Int
              author   User    @relation(fields: [authorId], references: [uniq])
            
              commentId Int
              comment   Comment @relation(fields: [commentId], references: [id])
            }
            
            model Comment {
              #id(id, Int, @id)
              post        Post?
              writtenById Int
              writtenBy   User  @relation(fields: [writtenById], references: [uniq])
            }"#
        };

        schema.to_owned()
    }

    // Updating foreign keys to a record that exist should work
    #[connector_test]
    async fn referenced_records_exist_one(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1
              uniq_1: 1
              uniq_2: 1
              post: { create: { id: 1 } }
              comments: { create: [
                { id: 1, post: { connect: { id: 1 } } },
                { id: 2, post: { connect: { id: 1 } } }
              ] }
            }"#,
        )
        .await?;

        create_row(
            &runner,
            r#"{
                id: 2
                uniq_1: 3
                uniq_2: 1
                post: { create: { id: 3 } }
                comments: { create: [
                  { id: 3, post: { connect: { id: 3 } } }
                ]}
            }"#,
        )
        .await?;

        // Update works because there's an existing:
        // - `User` with [uniq_1: 3, uniq_2: 1]
        // - `Post` with id: 3
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneComment(where: { id: 1 }, data: { writtenById_1: 3 }) { id writtenBy { uniq_1 uniq_2 } } }"#),
          @r###"{"data":{"updateOneComment":{"id":1,"writtenBy":{"uniq_1":3,"uniq_2":1}}}}"###
        );

        Ok(())
    }

    // Updating foreign keys (of many records) to a record that exist should work
    #[connector_test]
    async fn referenced_records_exist_many(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
                  id: 1
                  uniq_1: 1
                  uniq_2: 1
                  post: { create: { id: 1 } }
                  comments: { create: [
                    { id: 1, post: { connect: { id: 1 } } },
                    { id: 2, post: { connect: { id: 1 } } }
                  ] }
                }"#,
        )
        .await?;

        create_row(
            &runner,
            r#"{
                    id: 2
                    uniq_1: 3
                    uniq_2: 1
                    post: { create: { id: 3 } }
                    comments: { create: [
                      { id: 3, post: { connect: { id: 3 } } }
                    ]}
                }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateManyComment(data: { writtenById_1: 3 }) { count } }"#),
          @r###"{"data":{"updateManyComment":{"count":3}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyUser { id comments { id } } }"#),
          @r###"{"data":{"findManyUser":[{"id":1,"comments":[]},{"id":2,"comments":[{"id":2},{"id":1},{"id":3}]}]}}"###
        );

        Ok(())
    }

    // Nested updating foreign keys to a record that exist should work
    #[connector_test(schema(nested))]
    async fn referenced_records_exist_nested(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
                  id: 1,
                  uniq: 1,
                  comments: {
                    create: {
                      id: 1,
                      post: {
                        create: {
                          id: 1
                          author: { connect: { id: 1 } }
                        }
                      }
                    }
                  }
                }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{
                id: 2,
                uniq: 2,
                comments: {
                  create: {
                    id: 2,
                    post: {
                      create: {
                        id: 2
                        author: { connect: { id: 2 } }
                      }
                    }
                  }
                }
              }"#,
        )
        .await?;

        // `User` with `uniq`: 2 exists
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneUser(
              where: { id: 1 },
              data: { post: { update: { comment: { update: { writtenById: 2 } } } } }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"updateOneUser":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyUser { id comments { id } } }"#),
          @r###"{"data":{"findManyUser":[{"id":1,"comments":[]},{"id":2,"comments":[{"id":1},{"id":2}]}]}}"###
        );

        Ok(())
    }

    // Updating foreign keys to a record that does not exist should fail
    #[connector_test]
    async fn violates_required_relation_one(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1
              uniq_1: 1
              uniq_2: 1
              post: { create: { id: 1 } }
              comments: { create: [
                { id: 1, post: { connect: { id: 1 } } },
                { id: 2, post: { connect: { id: 1 } } }
              ] }
            }"#,
        )
        .await?;

        create_row(
            &runner,
            r#"{
              id: 2
              uniq_1: 3
              uniq_2: 1
              post: { create: { id: 2 } }
              comments: { create: [
                { id: 3, post: { connect: { id: 2 } } }
              ]}
            }"#,
        )
        .await?;

        // Can fail on `Comment.writtenBy` or `Comment.post` since there is
        // - no `User` with [uniq_1: 5, uniq_2: 1]
        // - nor `Post` with `id: 5`
        assert_error!(
            runner,
            r#"mutation { updateOneComment(where: { id: 1 }, data: { writtenById_1: 5 }) { id } }"#,
            2014,
            "The change you are trying to make would violate the required relation"
        );

        // Does _not_ fail on `Comment.writtenBy` but on `Comment.post` since there _is_ a `User` with [uniq_1: 3, uniq_2: 1]
        // but no `Post` with `id: 3`
        assert_error!(
            runner,
            r#"mutation { updateOneComment(where: { id: 1 }, data: { writtenById_1: 3 }) { id } }"#,
            2014,
            "The change you are trying to make would violate the required relation 'CommentToPost' between the `Comment` and `Post` models."
        );

        // Does _not_ fail on `Comment.post` since `Comment.writtenById_2` is not part of its fks
        assert_error!(
          runner,
          r#"mutation { updateOneComment(where: { id: 1 }, data: { writtenById_2: 5 }) { id } }"#,
          2014,
          "The change you are trying to make would violate the required relation 'CommentToUser' between the `Comment` and `User` models."
      );

        Ok(())
    }

    // Updating foreign keys (of many records) to a record that does not exist should fail
    #[connector_test]
    async fn violates_required_relation_many(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
                  id: 1
                  uniq_1: 1
                  uniq_2: 1
                  post: { create: { id: 1 } }
                  comments: { create: [
                    { id: 1, post: { connect: { id: 1 } } },
                    { id: 2, post: { connect: { id: 1 } } }
                  ] }
                }"#,
        )
        .await?;

        create_row(
            &runner,
            r#"{
                  id: 2
                  uniq_1: 3
                  uniq_2: 1
                  post: { create: { id: 2 } }
                  comments: { create: [
                    { id: 3, post: { connect: { id: 2 } } }
                  ]}
                }"#,
        )
        .await?;

        // Can fail on `Comment.writtenBy` or `Comment.post` since there is
        // - no `User` with [uniq_1: 5, uniq_2: 1]
        // - nor `Post` with `id: 5`
        assert_error!(
            runner,
            r#"mutation { updateManyComment(data: { writtenById_1: 5 }) { count } }"#,
            2014,
            "The change you are trying to make would violate the required relation"
        );

        // Does _not_ fail on `Comment.writtenBy` but on `Comment.post` since there _is_ a `User` with [uniq_1: 3, uniq_2: 1]
        // but no `Post` with `id: 3`
        assert_error!(
                runner,
                r#"mutation { updateManyComment(data: { writtenById_1: 3 }) { count } }"#,
                2014,
                "The change you are trying to make would violate the required relation 'CommentToPost' between the `Comment` and `Post` models."
            );

        // Does _not_ fail on `Comment.post` since `Comment.writtenById_2` is not part of its fks
        assert_error!(
              runner,
              r#"mutation { updateManyComment(data: { writtenById_2: 5 }) { count } }"#,
              2014,
              "The change you are trying to make would violate the required relation 'CommentToUser' between the `Comment` and `User` models."
          );

        Ok(())
    }

    // Nested updating foreign keys to a record that does not exist should fail
    #[connector_test(schema(nested))]
    async fn violates_required_relation_nested(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
                    id: 1,
                    uniq: 1,
                    comments: {
                      create: {
                        id: 1,
                        post: {
                          create: {
                            id: 1
                            author: { connect: { id: 1 } }
                          }
                        }
                      }
                    }
                  }"#,
        )
        .await?;

        assert_error!(
            runner,
            r#"mutation {
              updateOneUser(
                where: { id: 1 },
                data: { post: { update: { comment: { update: { writtenById: 2 } } } } }
              ) {
                id
              }
            }"#,
            2014,
            "The change you are trying to make would violate the required relation 'CommentToUser' between the `Comment` and `User` models"
        );

        Ok(())
    }

    // Updating foreign keys with anything else than `set` should not trigger emulation
    #[connector_test]
    async fn no_support_for_complex_updates_one(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1
              uniq_1: 1
              uniq_2: 1
              post: { create: { id: 1 } }
              comments: { create: [
                { id: 1, post: { connect: { id: 1 } } },
                { id: 2, post: { connect: { id: 1 } } }
              ] }
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneComment(where: { id: 1 }, data: { writtenById_1: { increment: 10 } }) { id writtenById_1 } }"#),
          @r###"{"data":{"updateOneComment":{"id":1,"writtenById_1":11}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneComment(where: { id: 1 }, data: { writtenById_1: { decrement: 1 } }) { id writtenById_1 } }"#),
          @r###"{"data":{"updateOneComment":{"id":1,"writtenById_1":10}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneComment(where: { id: 1 }, data: { writtenById_1: { divide: 2 } }) { id writtenById_1 } }"#),
          @r###"{"data":{"updateOneComment":{"id":1,"writtenById_1":5}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneComment(where: { id: 1 }, data: { writtenById_1: { multiply: 2 } }) { id writtenById_1 } }"#),
          @r###"{"data":{"updateOneComment":{"id":1,"writtenById_1":10}}}"###
        );

        Ok(())
    }

    // Updating foreign keys (of many records) with anything else than `set` should not trigger emulation
    #[connector_test]
    async fn no_support_for_complex_updates_many(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
                  id: 1
                  uniq_1: 1
                  uniq_2: 1
                  post: { create: { id: 1 } }
                  comments: { create: [
                    { id: 1, post: { connect: { id: 1 } } },
                    { id: 2, post: { connect: { id: 1 } } }
                  ] }
                }"#,
        )
        .await?;

        run_query!(
            &runner,
            r#"mutation { updateManyComment(data: { writtenById_1: { increment: 10 } }) { count } }"#
        );
        run_query!(
            &runner,
            r#"mutation { updateManyComment(data: { writtenById_1: { decrement: 1 } }) { count } }"#
        );
        run_query!(
            &runner,
            r#"mutation { updateManyComment(data: { writtenById_1: { divide: 2 } }) { count } }"#
        );
        run_query!(
            &runner,
            r#"mutation { updateManyComment(data: { writtenById_1: { multiply: 2 } }) { count } }"#
        );

        Ok(())
    }

    // Nested updating foreign keys with anything else than `set` should not trigger emulation
    #[connector_test(schema(nested))]
    async fn no_support_for_complex_updates_nested(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
                      id: 1,
                      uniq: 1,
                      comments: {
                        create: {
                          id: 1,
                          post: {
                            create: {
                              id: 1
                              author: { connect: { id: 1 } }
                            }
                          }
                        }
                      }
                  }"#,
        )
        .await?;

        run_query!(
            &runner,
            r#"mutation {
              updateOneUser(
                where: { id: 1 },
                data: { post: { update: { comment: { update: { writtenById: { increment: 10 } } } } } }
              ) {
                id
              }
            }"#
        );

        run_query!(
            &runner,
            r#"mutation {
              updateOneUser(
                where: { id: 1 },
                data: { post: { update: { comment: { update: { writtenById: { decrement: 1 } } } } } }
              ) {
                id
              }
          }"#
        );

        run_query!(
            &runner,
            r#"mutation {
              updateOneUser(
                where: { id: 1 },
                data: { post: { update: { comment: { update: { writtenById: { divide: 2 } } } } } }
              ) {
                id
              }
            }"#
        );

        run_query!(
            &runner,
            r#"mutation {
              updateOneUser(
                where: { id: 1 },
                data: { post: { update: { comment: { update: { writtenById: { multiply: 2 } } } } } }
              ) {
                id
              }
            }"#
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneUser(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
