//! Only Postgres (except CockroachDB) allows SetNull on a non-nullable FK at all, rest fail during migration.
//! D1 also seems to silently ignore Restrict.

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(suite = "setnull_onU_1to1_opt", schema(optional), relation_mode = "prisma")]
mod one2one_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String? @unique
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String? @unique
                parent      Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetNull)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent suceeds and sets the FK null.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn upsert_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, uniq: "1" }) { id }}"#),
          @r###"{"data":{"upsertOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_many_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#),
          @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
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
              a A? @relation(fields: [a_id], references: [b_id], onUpdate: SetNull)

              c C?
            }
            
            model C {
              #id(id, Int, @id)
              b_id Int? @unique
              b B? @relation(fields: [b_id], references: [a_id])
            }"#
        };

        schema.to_owned()
    }

    // SET_NULL should recurse if there are relations sharing a common fk
    #[connector_test(schema(one2one2one_opt_set_null))]
    async fn update_parent_recurse_set_null(runner: Runner) -> TestResult<()> {
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
          run_query!(runner, r#"mutation { updateOneA(where: { id: 1 }, data: { b_id: 2 }) { id } }"#),
          @r###"{"data":{"updateOneA":{"id":1}}}"###
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

    fn one2one2one_opt_restrict() -> String {
        let schema = indoc! {
            r#"model A {
            #id(id, Int, @id)
            b_id Int? @unique
            b B?
          }
          
          model B {
            #id(id, Int, @id)
            a_id Int? @unique
            a A? @relation(fields: [a_id], references: [b_id], onUpdate: SetNull)

            c C?
          }
          
          model C {
            #id(id, Int, @id)
            b_id Int? @unique
            b B? @relation(fields: [b_id], references: [a_id], onUpdate: Restrict)
          }"#
        };

        schema.to_owned()
    }

    // SET_NULL should recurse if there are relations sharing a common fk
    #[connector_test(schema(one2one2one_opt_restrict), exclude(SqlServer))]
    async fn update_parent_recurse_restrict_failure(runner: Runner) -> TestResult<()> {
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

        let query = r#"mutation { updateOneA(where: { id: 1 }, data: { b_id: 2 }) { id } }"#;

        assert_error!(
          runner,
          query,
          2014,
          "The change you are trying to make would violate the required relation 'BToC' between the `C` and `B` models."
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyA { id b_id b { id a_id c { id b_id } } } }"#),
          @r###"{"data":{"findManyA":[{"id":1,"b_id":1,"b":{"id":1,"a_id":1,"c":{"id":1,"b_id":1}}}]}}"###
        );

        Ok(())
    }

    fn one2one2one_no_shared_fk() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
            
              b_id Int? @unique
              b    B?
            }
            
            model B {
              #id(id, Int, @id)
            
              a_id Int? @unique
              c_id Int? @unique
            
              a A? @relation(fields: [a_id], references: [b_id], onUpdate: SetNull)
              c C?
            }
            
            model C {
              #id(id, Int, @id)
            
              b_id Int? @unique
              b    B?   @relation(fields: [b_id], references: [c_id], onUpdate: SetNull)
            }"#
        };

        schema.to_owned()
    }

    // SET_NULL should not recurse if there is no relation sharing a common fk
    #[connector_test(schema(one2one2one_no_shared_fk))]
    async fn update_parent_no_recursion(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneA(data: {
            id: 1,
            b_id: 1,
            b: {
              create: {
                id: 1,
                c_id: 1,
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
          run_query!(runner, r#"mutation { updateOneA(where: { id: 1 }, data: { b_id: 2 }) { id } }"#),
          @r###"{"data":{"updateOneA":{"id":1}}}"###
        );

        // B should be nulled
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyA { id b_id b { id } } }"#),
          @r###"{"data":{"findManyA":[{"id":1,"b_id":2,"b":null}]}}"###
        );

        // But C should not because it doesn't share a fk with the A->B relation
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyC { id } }"#),
          @r###"{"data":{"findManyC":[{"id":1}]}}"###
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
            parent           Parent? @relation(fields: [childUniq], references: [uniq], onUpdate: SetNull)
          }"#
        };

        schema.to_owned()
    }

    // Updating the parent updates the child FK as well.
    // Checks that it works even with different parent/child primary identifier names.
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
          @r###"{"data":{"updateOneParent":{"id":1,"uniq":2,"child":null}}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "setnull_onU_1toM_opt", schema(optional), relation_mode = "prisma")]
mod one2many_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq     String? @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String?
                parent    Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetNull)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent succeeds and sets the FK null.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }

    /// Updating the parent succeeds and sets the FK null.
    #[connector_test]
    async fn update_parent_nested(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneChild(where: { id: 1 }, data: { parent: { update: { uniq: "u1" } } }) { id }}"#),
          @r###"{"data":{"updateOneChild":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn upsert_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, uniq: "1", children: { create: { id: 1 }} }) { id }}"#),
          @r###"{"data":{"upsertOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn upsert_parent_nested(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneChild(
              where: { id: 1 }
              data: {
                parent: { upsert: { update: { uniq: "u1" }, create: { id: 3, uniq: "3" } } }
              }
          ) { id }}"#),
          @r###"{"data":{"updateOneChild":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_many_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#),
          @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
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
              parent        Parent? @relation(fields: [parent_uniq_1, parent_uniq_2], references: [uniq_1, uniq_2], onUpdate: SetNull)
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
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq_1":null,"parent_uniq_2":null}]}}"###
        );

        Ok(())
    }

    fn one2m2m_opt_set_null() -> String {
        let schema = indoc! {
            r#"model A {
            #id(id, Int, @id)

            b_id Int? @unique
            bs B[]
          }
          
          model B {
            #id(id, Int, @id)

            a_id Int? @unique
            a A? @relation(fields: [a_id], references: [b_id], onUpdate: SetNull)

            cs C[]
          }
          
          model C {
            #id(id, Int, @id)

            b_id Int? @unique
            b B? @relation(fields: [b_id], references: [a_id])
          }"#
        };

        schema.to_owned()
    }

    // SET_NULL should recurse if there are relations sharing a common fk
    #[connector_test(schema(one2m2m_opt_set_null))]
    async fn update_parent_recurse_set_null(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
          createOneA(data: {
            id: 1,
            b_id: 1,
            bs: { 
              create: {
                id: 1,
                cs: {
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
          run_query!(runner, r#"mutation { updateOneA(where: { id: 1 }, data: { b_id: 2 }) { id } }"#),
          @r###"{"data":{"updateOneA":{"id":1}}}"###
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

    fn one2m2m_opt_restrict() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
  
              b_id Int? @unique
              bs B[]
            }
            
            model B {
              #id(id, Int, @id)
  
              a_id Int? @unique
              a A? @relation(fields: [a_id], references: [b_id], onUpdate: SetNull)
  
              cs C[]
            }
            
            model C {
              #id(id, Int, @id)
  
              b_id Int? @unique
              b B? @relation(fields: [b_id], references: [a_id], onUpdate: Restrict)
            }"#
        };

        schema.to_owned()
    }

    // SET_NULL should recurse if there are relations sharing a common fk
    #[connector_test(schema(one2m2m_opt_restrict), exclude(SqlServer))]
    async fn update_parent_recurse_restrict_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
        createOneA(data: {
          id: 1,
          b_id: 1,
          bs: {
            create: {
              id: 1,
              cs: {
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

        let query = r#"mutation { updateOneA(where: { id: 1 }, data: { b_id: 2 }) { id } }"#;

        assert_error!(
          runner,
          query,
          2014,
          "The change you are trying to make would violate the required relation 'BToC' between the `C` and `B` models."
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyA { id b_id bs { id a_id cs { id b_id } } } }"#),
          @r###"{"data":{"findManyA":[{"id":1,"b_id":1,"bs":[{"id":1,"a_id":1,"cs":[{"id":1,"b_id":1}]}]}]}}"###
        );

        Ok(())
    }

    fn one2m2m_no_shared_fk() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
            
              b_id Int? @unique
              bs   B[]
            }
            
            model B {
              #id(id, Int, @id)
            
              a_id Int? @unique
              c_id Int? @unique
            
              a  A?  @relation(fields: [a_id], references: [b_id], onUpdate: SetNull)
              cs C[]
            }
            
            model C {
              #id(id, Int, @id)
            
              b_id Int? @unique
              b    B?   @relation(fields: [b_id], references: [c_id], onUpdate: SetNull)
            }
            "#
        };

        schema.to_owned()
    }

    // SET_NULL should not recurse if there is no relation sharing a common fk
    #[connector_test(schema(one2m2m_no_shared_fk))]
    async fn update_parent_no_recursion(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
        createOneA(data: {
          id: 1,
          b_id: 1,
          bs: {
            create: {
              id: 1,
              c_id: 1,
              cs: {
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
          run_query!(runner, r#"mutation { updateOneA(where: { id: 1 }, data: { b_id: 2 }) { id } }"#),
          @r###"{"data":{"updateOneA":{"id":1}}}"###
        );

        // B should be nulled
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyA { id b_id bs { id } } }"#),
          @r###"{"data":{"findManyA":[{"id":1,"b_id":2,"bs":[]}]}}"###
        );

        // But C should not because it doesn't share a fk with the A->B relation
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyC { id } }"#),
          @r###"{"data":{"findManyC":[{"id":1}]}}"###
        );

        Ok(())
    }

    fn one2m2m_compound_opt_set_null() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
              name String?
            
              b_id_1 Int?
              b_id_2 Int?
            
              bs B[]
            
              @@unique([b_id_1, b_id_2])
            }
            
            model B {
              #id(id, Int, @id)
              name String?
            
              a_id_1 Int?
              a_id_2 Int?
              a      A?   @relation(fields: [a_id_1, a_id_2], references: [b_id_1, b_id_2], onUpdate: SetNull)
            
              cs C[]
            
              @@unique([a_id_1, a_id_2])
            }
            
            model C {
              #id(id, Int, @id)
              name String?
            
              b_id_1 Int? @unique
              b_id_2 Int? @unique
              b      B?   @relation(fields: [b_id_1, b_id_2], references: [a_id_1, a_id_2])
            }
            "#
        };

        schema.to_owned()
    }

    // Relation fields with at least one shared compound should also be set to null
    #[connector_test(schema(one2m2m_compound_opt_set_null))]
    async fn update_parent_compound_recurse(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneA(data: {
              id: 1,
              b_id_1: 1,
              b_id_2: 1,
              bs: {
                create: {
                  id: 1,
                  cs: {
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

        // Update one of the compound unique
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { updateOneA(where: { id: 1 }, data: { b_id_1: 2 }) { id } } "#),
          @r###"{"data":{"updateOneA":{"id":1}}}"###
        );

        // Check that no Bs are connected to A anymore
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyA { id b_id_1 b_id_2 bs { id } } }"#),
          @r###"{"data":{"findManyA":[{"id":1,"b_id_1":2,"b_id_2":1,"bs":[]}]}}"###
        );

        // Check that both a_id_1 & a_id_2 compound were NULLed
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyB { id a_id_1 a_id_2 } }"#),
          @r###"{"data":{"findManyB":[{"id":1,"a_id_1":null,"a_id_2":null}]}}"###
        );

        // Check that both b_id_1 & b_id_2 compound were NULLed
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyC { id b_id_1 b_id_2 } }"#),
          @r###"{"data":{"findManyC":[{"id":1,"b_id_1":null,"b_id_2":null}]}}"###
        );

        Ok(())
    }
}
