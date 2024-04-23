//! Only Postgres (except CockroachDB) allows SetNull on a non-nullable FK at all, rest fail during migration.

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(suite = "setnull_onD_1to1_opt", schema(optional), relation_mode = "prisma")]
mod one2one_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int? @unique
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: SetNull)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent suceeds and sets the FK null.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
          @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_id }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_id":null}]}}"###
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
            parent           Parent? @relation(fields: [childUniq], references: [uniq], onDelete: SetNull)
          }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent suceeds and sets the FK null.
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
          @r###"{"data":{"findManyChild":[{"childUniq":null}]}}"###
        );

        Ok(())
    }

    fn one2one2one_opt_set_null() -> String {
        let schema = indoc! {
          r#"model A {
            #id(id, Int, @id)
            b_id Int? @unique
            b B?
          }

          model B {
            #id(id, Int, @id)
            a_id Int? @unique
            a A? @relation(fields: [a_id], references: [b_id], onDelete: SetNull)

            c C?
          }

          model C {
            #id(id, Int, @id)
            b_id Int? @unique
            b B? @relation(fields: [b_id], references: [a_id], onUpdate: SetNull)
          }
          "#
        };

        schema.to_owned()
    }

    // SET_NULL should also apply to child relations sharing a common fk
    #[connector_test(schema(one2one2one_opt_set_null))]
    async fn delete_parent_recurse_set_null(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneA(data: {
              id: 1,
              b_id: 1,
              b: {
                create: {
                  id: 1,
                  c: {
                    create: {
                      id: 1
                    }
                  }
                }
              }
            }) {
              id
            }
          }"#),
          @r###"{"data":{"createOneA":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneA(where: { id: 1 }) { id } }"#),
          @r###"{"data":{"deleteOneA":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyB { id a_id } }"#),
          @r###"{"data":{"findManyB":[{"id":1,"a_id":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyC { id b_id } }"#),
          @r###"{"data":{"findManyC":[{"id":1,"b_id":null}]}}"###
        );

        Ok(())
    }

    fn one2one2one_opt_set_null_restrict() -> String {
        let schema = indoc! {
            r#"model A {
                #id(id, Int, @id)
                b_id Int? @unique
                b B?
              }

              model B {
                #id(id, Int, @id)
                a_id Int? @unique
                a A? @relation(fields: [a_id], references: [b_id], onDelete: SetNull)

                c C?
              }

              model C {
                #id(id, Int, @id)
                b_id Int? @unique
                b B? @relation(fields: [b_id], references: [a_id], onDelete: SetNull, onUpdate: Restrict)
              }
            "#
        };

        schema.to_owned()
    }

    // SET_NULL should also apply to child relations sharing a common fk
    #[connector_test(schema(one2one2one_opt_set_null_restrict))]
    async fn delete_parent_set_null_restrict(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneA(data: {
              id: 1,
              b_id: 1,
              b: {
                create: {
                  id: 1,
                  c: {
                    create: {
                      id: 1
                    }
                  }
                }
              }
            }) {
              id
            }
          }"#),
          @r###"{"data":{"createOneA":{"id":1}}}"###
        );

        // Deletion of A fails because it updates B which in turns update C, except that C.b has an onUpdate: Restrict that's set.
        // So the entire operation cannot be performed.
        // Note that the `onDelete` referential action set on `C.b` is not taken into account here.
        assert_error!(
            runner,
            r#"mutation { deleteOneA(where: { id: 1 }) { id } }"#,
            2014,
            "The change you are trying to make would violate the required relation 'BToC' between the `C` and `B` models."
        );

        Ok(())
    }

    fn one2one2one_opt_set_null_cascade() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
              b_id Int? @unique
              b B?
            }

            model B {
              #id(id, Int, @id)
              a_id Int? @unique
              a A? @relation(fields: [a_id], references: [b_id], onDelete: SetNull)

              c C?
            }

            model C {
              #id(id, Int, @id)
              b_id Int? @unique
              b B? @relation(fields: [b_id], references: [a_id], onDelete: Restrict, onUpdate: Cascade)
            }
          "#
        };

        schema.to_owned()
    }

    // SET_NULL should also apply to child relations sharing a common fk
    #[connector_test(schema(one2one2one_opt_set_null_cascade), exclude_features("relationJoins"))]
    async fn delete_parent_set_null_cascade(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneA(data: {
              id: 1,
              b_id: 1,
              b: {
                create: {
                  id: 1,
                  c: {
                    create: {
                      id: 1
                    }
                  }
                }
              }
            }) {
              id
            }
          }"#),
          @r###"{"data":{"createOneA":{"id":1}}}"###
        );

        // Note that the `onDelete: Restrict` referential action set on field `C.b` is not taken into account here,
        // because B is never deleted but only updated based on the onDelete: SetNull referential action set on field `B.a`.
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneA(where: { id: 1 }) { id } }"#),
          @r###"{"data":{"deleteOneA":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyB { id a_id } }"#),
          @r###"{"data":{"findManyB":[{"id":1,"a_id":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyC { id b_id } }"#),
          @r###"{"data":{"findManyC":[{"id":1,"b_id":null}]}}"###
        );

        Ok(())
    }
}

#[test_suite(
    suite = "setnull_onD_1toM_opt",
    schema(optional),
    exclude(MongoDb),
    relation_mode = "prisma"
)]
mod one2many_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int?
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: SetNull)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent suceeds and sets the FK null.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
          @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_id }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_id":null}]}}"###
        );

        Ok(())
    }

    fn prisma_17255_schema() -> String {
        let schema = indoc! {
            r#"model Main {
            #id(id, Int, @id)

            alice   Alice?  @relation(fields: [aliceId], references: [id], onDelete: SetNull, onUpdate: Cascade)
            aliceId Int?

            bob Bob?
          }

          model Alice {
            #id(id, Int, @id)
            manyMains Main[]
          }

          model Bob {
            #id(id, Int, @id)

            main   Main   @relation(fields: [mainId], references: [id], onDelete: Cascade, onUpdate: Cascade)
            mainId Int @unique
          }"#
        };

        schema.to_owned()
    }

    // Do not recurse when relations have no fks in common
    #[connector_test(schema(prisma_17255_schema))]
    async fn prisma_17255(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
          createOneMain(data: {
            id: 1,
            alice: { create: { id: 1 } },
            bob: { create: { id: 1 } }
          }) {
            id
          }
        }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneMain(where: { id: 1 }, data: {
              alice: { delete: true }
            }) {
              id
            }
          }"#),
          @r###"{"data":{"updateOneMain":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyMain { id alice { id } bob { id } } }"#),
          @r###"{"data":{"findManyMain":[{"id":1,"alice":null,"bob":{"id":1}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlice { id } }"#),
          @r###"{"data":{"findManyAlice":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyBob { id } }"#),
          @r###"{"data":{"findManyBob":[{"id":1}]}}"###
        );

        Ok(())
    }
}
