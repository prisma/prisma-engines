use query_engine_tests::*;

// Note: These tests changed from including the relation fields into only including the scalars as per the new relations
// implementation. Tests are retained as they offer a good coverage over scalar + relation field usage.
//
// 1) Checks if relation fields in @@unique in any constellation work with our mutations.
// Possible relation cardinalities:
// - 1!:1!
// - 1!:1
// - 1!:M
//
// 2) Checks basic cursor functionality.
#[test_suite]
mod compound_uniq_rel_field {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)
              p        String
              child_id Int

              child Child  @relation(fields: [child_id], references: [id])
              @@unique([child_id, p])
            }

            model Child {
              #id(id, Int, @id)
              c      String
              parent Parent?

              @@unique([id, c])
            }"#
        };

        schema.to_owned()
    }

    // Mutations in this test:
    //  create         | root   | checked
    //  update         | root   | checked
    //  delete         | root   | checked
    //  upsert         | root   | checked
    //  updateMany     | root   | unnecessary
    //  deleteMany     | root   | unnecessary
    //  nested create  | create | checked
    //  nested update  | update | checked
    //  nested connect | create | checked
    //  nested connect | update | checked
    //  nested delete  | -      | checked
    //  nested upsert  | update | checked
    //  nested disconn | -      | not possible (1!:1)
    //  nested set     | -      | not possible (1!:1)
    //  nested deleteM | -      | not possible (1!:1)
    //  nested updateM | -      | not possible (1!:1)
    // "Using a compound unique that includes a 1!:1 single-field relation" should "work"
    #[connector_test(schema(schema_1))]
    async fn compound_uniq_with_1_1_single_rel(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: { id: 1, p: "Parent1", child: { create: { id: 1, c: "Child1" }}}) {
              p
              child {
                 c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent1","child":{"c":"Child1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(where: { child_id_p: { child_id: 1, p: "Parent1" } } data: { p: { set: "UpdatedParent1" }}) {
              p
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"p":"UpdatedParent1"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(where: { id: 1 } data: { parent: { update: { p: { set: "UpdateParent1FromChild" }}}}) {
              parent { p }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"parent":{"p":"UpdateParent1FromChild"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
              where: { child_id_p: { child_id: 2, p: "Parent2" } }
              update: { p: { set: "doesn't matter" }}
              create: { id: 2, p: "Parent2", child: { create: { id: 2, c: "Child2" } } }
            ) {
              p
            }
          }"#),
          @r###"{"data":{"upsertOneParent":{"p":"Parent2"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteOneParent(
              where: {
                child_id_p: { child_id: 2, p: "Parent2" }
              }
            ) {
              p
            }
          }"#),
          @r###"{"data":{"deleteOneParent":{"p":"Parent2"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(
              data: {
                id: 2
                p: "Parent2New",
                child: {
                  connect: {
                    id: 2
                  }
                }
              }
            ) {
              p
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent2New","child":{"id":2}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 3
                c: "Child3",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":3}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(
              where: {
                child_id_p: { child_id: 2, p: "Parent2New" }
              }
              data: {
                child: {
                  connect: {
                    id: 3
                  }
                }
              }
            ) {
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"child":{"id":3}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parent: {
                  upsert: {
                    create: {
                      id: 3
                      p: "Parent3",
                    }
                    update: {
                      p: { set: "doesn't matter" }
                    }
                  }
                }
              }
            ) {
              id
              parent {
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":3,"parent":{"child":{"id":3}}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parent: {
                  delete: true
                }
              }
            ) {
              id
              parent {
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":3,"parent":null}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)
              p        String
              child_id Int
              child_c  String

              child Child  @relation(fields: [child_id, child_c], references: [id, c])
              @@unique([child_id, child_c, p])
            }

            model Child {
              #id(id, Int, @id)
              c      String
              parent Parent?

              @@unique([id, c])
            }"#
        };

        schema.to_owned()
    }

    // Mutations in this test:
    //  create         | root   | checked
    //  update         | root   | checked
    //  delete         | root   | checked
    //  upsert         | root   | checked
    //  updateMany     | root   | unnecessary
    //  deleteMany     | root   | unnecessary
    //  nested create  | create | checked
    //  nested update  | update | checked
    //  nested connect | create | checked
    //  nested connect | update | checked
    //  nested delete  | -      | checked
    //  nested upsert  | update | checked
    //  nested disconn | -      | not possible (1!:1)
    //  nested set     | -      | not possible (1!:1)
    //  nested deleteM | -      | not possible (1!:1)
    //  nested updateM | -      | not possible (1!:1)
    // "Using a compound unique that includes a 1!:1 multi-field relation"
    #[connector_test(schema(schema_2))]
    async fn compound_uniq_with_1_1_multi_rel(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: { id: 1, p: "Parent1", child: { create: { id: 1, c: "Child1" }}}) {
              p
              child {
                 c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent1","child":{"c":"Child1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(where: { child_id_child_c_p: { child_id: 1, child_c: "Child1", p: "Parent1" } } data: { p: { set: "UpdatedParent1" }}) {
              p
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"p":"UpdatedParent1"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(where: { id: 1 } data: { parent: { update: { p: { set: "UpdateParent1FromChild" }}}}) {
              parent { p }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"parent":{"p":"UpdateParent1FromChild"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
              where: { child_id_child_c_p: { child_id: 2, child_c: "Child2", p: "Parent2" } }
              update: { p: { set: "doesn't matter" }}
              create: { id: 2, p: "Parent2", child: { create: { id: 2, c: "Child2" } } }
            ) {
              p
            }
          }"#),
          @r###"{"data":{"upsertOneParent":{"p":"Parent2"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteOneParent(
              where: {
                child_id_child_c_p: { child_id: 2, child_c: "Child2", p: "Parent2" }
              }
            ) {
              p
            }
          }"#),
          @r###"{"data":{"deleteOneParent":{"p":"Parent2"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(
              data: {
                id: 2
                p: "Parent2New",
                child: {
                  connect: {
                    id: 2
                  }
                }
              }
            ) {
              p
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent2New","child":{"id":2}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 3
                c: "Child3",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":3}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(
              where: {
                child_id_child_c_p: { child_id: 2, child_c: "Child2", p: "Parent2New" }
              }
              data: {
                child: {
                  connect: {
                    id: 3
                  }
                }
              }
            ) {
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"child":{"id":3}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 4
                c: "Child4",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":4}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 4 }
              data: {
                parent: {
                  upsert: {
                    create: {
                      id: 3
                      p: "Parent3",
                    }
                    update: {
                      p: { set: "doesn't matter" }
                    }
                  }
                }
              }
            ) {
              id
              parent {
                p
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":4,"parent":{"p":"Parent3","child":{"id":4}}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parent: {
                  delete: true
                }
              }
            ) {
              id
              parent {
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":3,"parent":null}}}"###
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)
              p        String
              child_id Int

              child Child  @relation(fields: [child_id], references: [id])
              @@unique([child_id, p])
            }

            model Child {
              #id(id, Int, @id)
              c       String
              parents Parent[]

              @@unique([id, c])
            }"#
        };

        schema.to_owned()
    }

    // Mutations in this test:
    //  create         | root   | checked
    //  update         | root   | checked
    //  delete         | root   | checked
    //  upsert         | root   | checked
    //  updateMany     | root   | unnecessary
    //  deleteMany     | root   | unnecessary
    //  nested create  | create | checked
    //  nested update  | update | checked
    //  nested connect | create | checked
    //  nested connect | update | checked
    //  nested delete  | -      | checked
    //  nested upsert  | update | checked
    //  nested deleteM | -      | checked
    //  nested updateM | -      | checked
    //  nested disconn | -      | not possible (1!:m)
    //  nested set     | -      | not (really) possible (1!:m)
    // "Using a compound unique that includes a 1!:M single-field relation"
    #[connector_test(schema(schema_3))]
    async fn compound_uniq_with_1_m_single_rel(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: { id: 1, p: "Parent1", child: { create: { id: 1, c: "Child1" }}}) {
              p
              child {
                 id
                 c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent1","child":{"id":1,"c":"Child1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(where: { child_id_p: { child_id: 1, p: "Parent1" } } data: { p: { set: "Parent1Updated" }}) {
              p
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"p":"Parent1Updated"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 1 }
              data: {
                parents: {
                  updateMany: {
                    where: { p: { equals: "Parent1Updated" } }
                    data: { p: { set: "Parent2UpdatedMany" } }
                  }
                }
              }
            ) {
              parents {
                p
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"parents":[{"p":"Parent2UpdatedMany"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
              where: { child_id_p: { child_id: 2, p: "Parent2" } }
              update: { p: { set: "doesn't matter" }}
              create: { id: 2, p: "Parent2", child: { create: { id: 2, c: "Child2" } } }
            ) {
              p
            }
          }"#),
          @r###"{"data":{"upsertOneParent":{"p":"Parent2"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteOneParent(
              where: { child_id_p: { child_id: 2, p: "Parent2" } }
            ) {
              p
            }
          }"#),
          @r###"{"data":{"deleteOneParent":{"p":"Parent2"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(
              data: {
                id: 2
                p: "Parent2New",
                child: {
                  connect: {
                    id: 2
                  }
                }
              }
            ) {
              p
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent2New","child":{"id":2}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 3
                c: "Child3",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":3}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(
              where: { child_id_p: { child_id: 2, p: "Parent2New" } }
              data: {
                child: {
                  connect: {
                    id: 3
                  }
                }
              }
            ) {
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"child":{"id":3}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 4
                c: "Child4",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":4}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 4 }
              data: {
                parents: {
                  upsert: {
                    where: { child_id_p: { child_id: 3, p: "Parent3" } }
                    create: { id: 3, p: "Parent3" }
                    update: { p: { set: "doesn't matter" }}
                  }
                }
              }
            ) {
              id
              parents {
                id
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":4,"parents":[{"id":3,"child":{"id":4}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parents: {
                  updateMany: {
                    where: { p: { equals: "Parent2New" }}
                    data: { p: { set: "Parent2NewUpdateMany" }}
                  }
                }
              }
            ) {
              id
              parents {
                p
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":3,"parents":[{"p":"Parent2NewUpdateMany","child":{"id":3}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parents: {
                  deleteMany: {
                    p: { equals: "Parent2NewUpdateMany" }
                  }
                }
              }
            ) {
              id
              parents {
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":3,"parents":[]}}}"###
        );

        Ok(())
    }

    fn schema_4() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)
              p        String
              child_id Int
              child_c  String

              child Child  @relation(fields: [child_id, child_c], references: [id, c])
              @@unique([child_id, child_c, p])
            }

            model Child {
              #id(id, Int, @id)
              c       String
              parents Parent[]

              @@unique([id, c])
            }"#
        };

        schema.to_owned()
    }

    // Mutations in this test:
    //  create         | root   | checked
    //  update         | root   | checked
    //  delete         | root   | checked
    //  upsert         | root   | checked
    //  updateMany     | root   | unnecessary
    //  deleteMany     | root   | unnecessary
    //  nested create  | create | checked
    //  nested update  | update | checked
    //  nested connect | create | checked
    //  nested connect | update | checked
    //  nested delete  | -      | checked
    //  nested upsert  | update | checked
    //  nested deleteM | -      | checked
    //  nested updateM | -      | checked
    //  nested disconn | -      | not possible (1!:m)
    //  nested set     | -      | not (really) possible (1!:m)
    // "Using a compound unique that includes a 1!:M multi-field relation"
    #[connector_test(schema(schema_4))]
    async fn compound_uniq_with_1_m_multi_rel(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: { id: 1, p: "Parent1", child: { create: { id: 1, c: "Child1" }}}) {
              p
              child {
                 id
                 c
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent1","child":{"id":1,"c":"Child1"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(where: { child_id_child_c_p: { child_id: 1, child_c: "Child1", p: "Parent1" } } data: { p: { set: "Parent1Updated" }}) {
              p
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"p":"Parent1Updated"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(where: { id: 1 } data: {
              parents: {
                 updateMany: {
                   where: { p: { equals: "Parent1Updated" }}
                   data: { p: { set: "Parent2UpdatedMany" }}}
                 }
               }
            ) {
              parents {
                p
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"parents":[{"p":"Parent2UpdatedMany"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
              where: { child_id_child_c_p: { child_id: 2, child_c: "Child2", p: "Parent2" } }
              update: { p: { set: "doesn't matter" }}
              create: { id: 2, p: "Parent2", child: { create: { id: 2, c: "Child2" } } }
            ) {
              p
            }
          }"#),
          @r###"{"data":{"upsertOneParent":{"p":"Parent2"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteOneParent(
              where: { child_id_child_c_p: { child_id: 2, child_c: "Child2", p: "Parent2" } }
            ) {
              p
            }
          }"#),
          @r###"{"data":{"deleteOneParent":{"p":"Parent2"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(
              data: {
                id: 2
                p: "Parent2New",
                child: {
                  connect: {
                    id: 2
                  }
                }
              }
            ) {
              p
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"p":"Parent2New","child":{"id":2}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 3
                c: "Child3",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":3}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(
              where: { child_id_child_c_p: { child_id: 2, child_c: "Child2", p: "Parent2New" } }
              data: {
                child: {
                  connect: {
                    id: 3
                  }
                }
              }
            ) {
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"child":{"id":3}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 4
                c: "Child4",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":4}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 4 }
              data: {
                parents: {
                  upsert: {
                    where: { child_id_child_c_p: { child_id: 3, child_c: "Child3", p: "Parent3" } }
                    create: { id: 3, p: "Parent3" }
                    update: { p: { set: "doesn't matter" }}
                  }
                }
              }
            ) {
              id
              parents {
                id
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":4,"parents":[{"id":3,"child":{"id":4}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parents: {
                  updateMany: {
                    where: { p: { equals: "Parent2New" }}
                    data: { p: { set: "Parent2NewUpdateMany" }}
                  }
                }
              }
            ) {
              id
              parents {
                p
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":3,"parents":[{"p":"Parent2NewUpdateMany","child":{"id":3}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parents: {
                  deleteMany: {
                    p: { equals: "Parent2NewUpdateMany" }
                  }
                }
              }
            ) {
              id
              parents {
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":3,"parents":[]}}}"###
        );

        Ok(())
    }

    fn schema_5() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              fieldA  String
              fieldB  String
              @@unique([fieldA, fieldB])
            }

            model ModelB {
              #id(id, Int, @id)
              fieldA  Int
              fieldB  Int

              @@unique([fieldA, fieldB])
            }"#
        };

        schema.to_owned()
    }

    // "Using compounds uniques that use the same field names in different models"
    #[connector_test(schema(schema_5))]
    async fn compound_uniq_same_field_diff_models(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#" mutation { createOneModelA(data: { id: 1, fieldA: "a", fieldB: "b" }) { id } }"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneModelB(data: { id: 1, fieldA: 1, fieldB: 2 }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueModelA(where: {
              fieldA_fieldB: {
                fieldA: "a",
                fieldB: "b"
              }
            }) { fieldA fieldB }
           }"#),
          @r###"{"data":{"findUniqueModelA":{"fieldA":"a","fieldB":"b"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueModelB(where: {
              fieldA_fieldB: {
                fieldA: 1,
                fieldB: 2
              }
            }) { fieldA fieldB }
           }"#),
          @r###"{"data":{"findUniqueModelB":{"fieldA":1,"fieldB":2}}}"###
        );

        Ok(())
    }
}
