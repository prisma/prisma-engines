use query_engine_tests::*;

#[test_suite(suite = "cascade_onD_1to1_req", schema(required), relation_mode = "prisma")]
mod one2one_req {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int @unique
                parent    Parent @relation(fields: [parent_id], references: [id], onDelete: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent deletes child as well.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyChild { id }}"),
            @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onD_1to1_opt", schema(optional), relation_mode = "prisma")]
mod one2one_opt {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int? @unique
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent deletes child as well.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyChild { id }}"),
            @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }

    fn diff_id_name() -> String {
        let schema = indoc! {
            r#"model Parent {
            #id(id, Int, @id)
            uniq    Int? @unique
            child   Child?
          }
          
          model Child {
            #id(childId, Int, @id)
            childUniq       Int? @unique
            parent           Parent? @relation(fields: [childUniq], references: [uniq], onDelete: Cascade)
          }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent deletes child as well.
    /// Checks that it works even with different parent/child primary identifier names.
    #[connector_test(schema(diff_id_name))]
    async fn delete_parent_diff_id_name(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneParent(data: { id: 1, uniq: 1, child: { create: { childId: 1 } } }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id } }"#),
          @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyChild { childUniq } }"#),
          @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onD_1toM_req", schema(required), relation_mode = "prisma")]
mod one2many_req {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int
                parent    Parent @relation(fields: [parent_id], references: [id], onDelete: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent deletes all children.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, children: { create: [ { id: 1 }, { id: 2 } ] }}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyChild { id }}"),
            @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onD_1toM_opt", schema(optional), relation_mode = "prisma")]
mod one2many_opt {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int?
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent deletes all children.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, children: { create: [ { id: 1 }, { id: 2 } ] }}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyChild { id }}"),
            @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(schema), exclude(SqlServer), relation_mode = "prisma")]
mod multiple_cascading_paths {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model User {
                #id(id, Int, @id)
                comments Comment[]
                posts    Post[]
              }
              
              model Post {
                #id(id, Int, @id)
                authorId Int
                author   User      @relation(fields: [authorId], references: [id], onDelete: Cascade)
                comments Comment[]
              }
              
              model Comment {
                #id(id, Int, @id)
                writtenById Int
                postId      Int
                writtenBy   User @relation(fields: [writtenById], references: [id], onDelete: Cascade)
                post        Post @relation(fields: [postId], references: [id], onDelete: Cascade)
              }
              "#
        };

        schema.to_owned()
    }

    // Ensure multiple cascading paths to the same model don't end up updating the same model twice and error out
    // The two paths are:
    //   - User -> Comment
    //   - User -> Post -> Comment
    #[connector_test]
    async fn should_work(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
            createOneUser(
              data: {
                id: 1
                posts: {
                  create: {
                    id: 1,
                    comments: {
                      create: {
                        id: 1,
                        writtenBy: {
                          connect: { id: 1 }
                        }
                      }
                    }
                  }
                }
              }
            ) {
              id
            }
          }
          "#
        );
        // A second user is created to ensure that it won't be touched by the cascade delete
        run_query!(
            &runner,
            r#"mutation {
              createOneUser(
                data: {
                  id: 2
                  posts: {
                    create: {
                      id: 2,
                      comments: {
                        create: {
                          id: 2,
                          writtenBy: {
                            connect: { id: 2 }
                          }
                        }
                      }
                    }
                  }
                }
              ) {
                id
              }
            }
          "#
        );

        run_query!(
            &runner,
            r#"mutation {
              deleteOneUser(where: { id: 1 }) {
                id
              }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser {
              id
            }
          }
          "#),
          @r###"{"data":{"findManyUser":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost {
              id
            }
          }
          "#),
          @r###"{"data":{"findManyPost":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyComment{
              id
            }
          }
          "#),
          @r###"{"data":{"findManyComment":[{"id":2}]}}"###
        );

        Ok(())
    }
}
