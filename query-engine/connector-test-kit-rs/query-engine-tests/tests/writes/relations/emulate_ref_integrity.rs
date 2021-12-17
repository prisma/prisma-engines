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

    // Updating foreign keys to a record that exist should work
    #[connector_test]
    async fn referenced_records_exist(runner: Runner) -> TestResult<()> {
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

    // Updating foreign keys to a record that does not exist should fail
    #[connector_test]
    async fn violates_required_relation(runner: Runner) -> TestResult<()> {
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

    // Updating foreign keys with anything else than `set` should not trigger emulation
    #[connector_test]
    async fn no_support_for_complex_updates(runner: Runner) -> TestResult<()> {
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
