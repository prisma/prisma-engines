//! D1 seems to silently ignore Cascade.
use query_engine_tests::*;

#[test_suite(suite = "cascade_onU_1to1_req", schema(required), relation_mode = "prisma")]
mod one2one_req {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                name String?
                uniq Int @unique
                child Child?
              }
              
              model Child {
                #id(id, Int, @id)
                parentUniq  Int @unique
                parent    Parent @relation(fields: [parentUniq], references: [uniq], onUpdate: Cascade)
                child2 Child2?
              }
              
              model Child2 {
                #id(id, Int, @id)
                childUniq Int @unique
                child   Child @relation(fields: [childUniq], references: [parentUniq], onUpdate: Cascade)
              }
              "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(required))]
    async fn update_parent_cascade(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation {
                createOneParent(data: {
                    id: 1,
                    uniq: 1,
                    child: {
                        create: {
                            id: 1,
                            child2: { create: { id: 1 } }
                        }
                    }
                }) { id }
            }"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: 2 }) { uniq } }"#),
          @r###"{"data":{"updateOneParent":{"uniq":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyParent { uniq child { parentUniq child2 { childUniq } } } }"#),
          @r###"{"data":{"findManyParent":[{"uniq":2,"child":{"parentUniq":2,"child2":{"childUniq":2}}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: 3 }) { count } }"#),
          @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyParent { uniq child { parentUniq child2 { childUniq } } } }"#),
          @r###"{"data":{"findManyParent":[{"uniq":3,"child":{"parentUniq":3,"child2":{"childUniq":3}}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: 4 }, create: { id: 1, uniq: 1 }) { uniq } }"#),
          @r###"{"data":{"upsertOneParent":{"uniq":4}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyParent { uniq child { parentUniq child2 { childUniq } } } }"#),
          @r###"{"data":{"findManyParent":[{"uniq":4,"child":{"parentUniq":4,"child2":{"childUniq":4}}}]}}"###
        );

        // Checks that it work when no FK is updated
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { upsertOneParent(where: { id: 1 }, update: { name: "Bob" }, create: { id: 1, uniq: 1 }) { name uniq } }"#),
          @r###"{"data":{"upsertOneParent":{"name":"Bob","uniq":4}}}"###
        );

        Ok(())
    }

    fn required_compound() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)
              uniq_1   String
              uniq_2   String
              child Child?
            
              @@unique([uniq_1, uniq_2])
            }
            
            model Child {
              #id(id, Int, @id)
              parent_uniq_1 String
              parent_uniq_2 String
              parent        Parent @relation(fields: [parent_uniq_1, parent_uniq_2], references: [uniq_1, uniq_2], onUpdate: Cascade)

              @@unique([parent_uniq_1, parent_uniq_2])
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(required_compound))]
    async fn update_parent_compound_cascade(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: { id: 1, uniq_1: "u1", uniq_2: "u2", child: { create: { id: 1 }}}) {
              id
            }
          }"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq_1: "u3" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq_1 parent_uniq_2 }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq_1":"u3","parent_uniq_2":"u2"}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onU_1to1_opt", schema(optional), relation_mode = "prisma")]
mod one2one_opt {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq Int? @unique
                childOpt Child?
              }
              
              model Child {
                #id(id, Int, @id)
                parentUniq  Int? @unique
                parent    Parent? @relation(fields: [parentUniq], references: [uniq], onUpdate: Cascade)
                child2Opt Child2?
              }
              
              model Child2 {
                #id(id, Int, @id)
                childUniq Int? @unique
                child   Child? @relation(fields: [childUniq], references: [parentUniq], onUpdate: Cascade)
              }
              "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(optional))]
    async fn update_parent_cascade(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation {
                createOneParent(data: {
                    id: 1,
                    uniq: 1,
                    childOpt: {
                        create: {
                            id: 1,
                            child2Opt: { create: { id: 1 } }
                        }
                    }
                }) { id }
            }"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: 2 }) { uniq } }"#),
          @r###"{"data":{"updateOneParent":{"uniq":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyParent { uniq childOpt { parentUniq child2Opt { childUniq } } } }"#),
          @r###"{"data":{"findManyParent":[{"uniq":2,"childOpt":{"parentUniq":2,"child2Opt":{"childUniq":2}}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: 3 }) { count } }"#),
          @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyParent { uniq childOpt { parentUniq child2Opt { childUniq } } } }"#),
          @r###"{"data":{"findManyParent":[{"uniq":3,"childOpt":{"parentUniq":3,"child2Opt":{"childUniq":3}}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: 4 }, create: { id: 1, uniq: 1 }) { uniq } }"#),
          @r###"{"data":{"upsertOneParent":{"uniq":4}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyParent { uniq childOpt { parentUniq child2Opt { childUniq } } } }"#),
          @r###"{"data":{"findManyParent":[{"uniq":4,"childOpt":{"parentUniq":4,"child2Opt":{"childUniq":4}}}]}}"###
        );

        Ok(())
    }

    fn optional_compound() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)
              name     String?
              uniq_1   String?
              uniq_2   String?
              child Child?
            
              @@unique([uniq_1, uniq_2])
            }
            
            model Child {
              #id(id, Int, @id)
              name          String?
              parent_uniq_1 String?
              parent_uniq_2 String?
              parent        Parent? @relation(fields: [parent_uniq_1, parent_uniq_2], references: [uniq_1, uniq_2], onUpdate: Cascade)

              @@unique([parent_uniq_1, parent_uniq_2])
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(optional_compound))]
    async fn update_parent_compound_cascade(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: { id: 1, uniq_1: "u1", uniq_2: "u2", child: { create: { id: 1 }}}) {
              id
            }
          }"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq_1: "u3" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq_1 parent_uniq_2 }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq_1":"u3","parent_uniq_2":"u2"}]}}"###
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
              parent           Parent? @relation(fields: [childUniq], references: [uniq], onUpdate: Cascade)
            }"#
        };

        schema.to_owned()
    }

    // Updating the parent updates the child FK as well.
    // Checks that it works even with different parent/child primary identifier names
    #[connector_test(schema(diff_id_name))]
    async fn update_parent_diff_id_name(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneParent(data: { id: 1, uniq: 1, child: { create: { childId: 1 } } }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(
              where: { id: 1 }
              data: { uniq: 2 }
            ) {
              id
              uniq
              child { childId childUniq }
            }
          }
          "#),
          @r###"{"data":{"updateOneParent":{"id":1,"uniq":2,"child":{"childId":1,"childUniq":2}}}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onU_1toM_req", schema(required), relation_mode = "prisma")]
mod one2many_req {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq     String @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String
                parent    Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent updates the child as well.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "1u" }) { uniq }}"#),
            @r###"{"data":{"updateOneParent":{"uniq":"1u"}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyParent { uniq children { parent_uniq } }}"),
            @r###"{"data":{"findManyParent":[{"uniq":"1u","children":[{"parent_uniq":"1u"}]}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onU_1toM_opt", schema(optional), relation_mode = "prisma")]
mod one2many_opt {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq     String  @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String?
                parent    Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent updates the child as well.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "1u" }) { uniq }}"#),
            @r###"{"data":{"updateOneParent":{"uniq":"1u"}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyParent { uniq children { parent_uniq } }}"),
            @r###"{"data":{"findManyParent":[{"uniq":"1u","children":[{"parent_uniq":"1u"}]}]}}"###
        );

        Ok(())
    }

    fn optional_compound_uniq() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)
              name     String?
              uniq_1   String?
              uniq_2   String?
              children Child[]
            
              @@unique([uniq_1, uniq_2])
            }
            
            model Child {
              #id(id, Int, @id)
              name          String?
              parent_uniq_1 String?
              parent_uniq_2 String?
              parent        Parent? @relation(fields: [parent_uniq_1, parent_uniq_2], references: [uniq_1, uniq_2], onUpdate: Cascade)
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(optional_compound_uniq))]
    async fn update_compound_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneParent(data: { id: 1, uniq_1: "u1", uniq_2: "u2", children: { create: { id: 1 }}}) {
              id
            }
          }"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq_1: "u3" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq_1 parent_uniq_2 }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq_1":"u3","parent_uniq_2":"u2"}]}}"###
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
                uniq Int @unique
                comments Comment[]
                posts    Post[]
              }
              
              model Post {
                #id(id, Int, @id)
                authorId Int
                author   User      @relation(fields: [authorId], references: [uniq], onUpdate: Cascade)
                comments Comment[]
              }
              
              model Comment {
                #id(id, Int, @id)
                writtenById Int
                postId      Int
                writtenBy   User @relation(fields: [writtenById], references: [uniq], onUpdate: Cascade)
                post        Post @relation(fields: [postId], references: [id], onUpdate: Cascade)
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
    async fn it_should_work(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
            createOneUser(
              data: {
                id: 1
                uniq: 1,
                posts: {
                  create: {
                    id: 1,
                    comments: {
                      create: {
                        id: 1,
                        writtenBy: {
                          connect: { uniq: 1 }
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
        // A second user is created to ensure that it won't be touched by the cascade update
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(
                  data: {
                    id: 3
                    uniq: 3,
                    posts: {
                      create: {
                        id: 3,
                        comments: {
                          create: {
                            id: 3,
                            writtenBy: {
                              connect: { uniq: 3 }
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
            updateOneUser(where: { id: 1 }, data: { uniq: 2 }) {
              id
            }
          }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser(orderBy: { id: asc }) {
              id
              uniq
              comments { id, writtenById }
              posts { id, authorId }
            }
          }
          "#),
          @r###"{"data":{"findManyUser":[{"id":1,"uniq":2,"comments":[{"id":1,"writtenById":2}],"posts":[{"id":1,"authorId":2}]},{"id":3,"uniq":3,"comments":[{"id":3,"writtenById":3}],"posts":[{"id":3,"authorId":3}]}]}}"###
        );

        Ok(())
    }
}
