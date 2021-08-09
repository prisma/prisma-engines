use query_engine_tests::*;

// Note: These tests changed from including the relation fields into only including the scalars as per the new relations
// implementation. Tests are retained as they offer a good coverage over scalar + relation field usage.
//
// 1) Checks if relation fields in @id in any constellation work with our mutations.
// Possible relation cardinalities:
// - 1!:1!
// - 1!:1
// - 1!:M
//
// 2) Checks basic cursor functionality.
#[test_suite]
mod single_pk_rel_field {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_1_1_single_rel() -> String {
        let schema = indoc! {
            r#"model Parent {
              name     String
              age      Int
              #id(child_id, Int, @id)

              child Child  @relation(fields: [child_id], references: [id])
            }

            model Child {
              #id(id, Int, @id)
              name    String
              parent  Parent?
            }"#
        };

        schema.to_owned()
    }

    fn schema_1_1_multi_rel() -> String {
        let schema = indoc! {
            r#"model Parent {
              name      String
              age       Int
              child_id  Int
              child_ssn String

              child Child  @relation(fields: [child_id, child_ssn], references: [id, ssn])
              @@id([child_id, child_ssn])
            }

            model Child {
              #id(id, Int, @id)
              ssn    String @unique
              name   String
              parent Parent?

              @@unique([id, ssn])
            }"#
        };

        schema.to_owned()
    }

    fn schema_1_m_single_rel() -> String {
        let schema = indoc! {
            r#"model Parent {
              name     String
              age      Int
              #id(child_id, Int, @id)

              child Child  @relation(fields: [child_id], references: [id])
            }

            model Child {
              #id(id, Int, @id)
              name    String
              parents Parent[]
            }"#
        };

        schema.to_owned()
    }

    fn schema_1_m_multi_rel() -> String {
        let schema = indoc! {
            r#"model Parent {
              name      String
              age       Int
              child_id  Int
              child_ssn String

              child Child  @relation(fields: [child_id, child_ssn], references: [id, ssn])
              @@id([child_id, child_ssn])
            }

            model Child {
              #id(id, Int, @id)
              ssn     String @unique
              name    String
              parents Parent[]

              @@unique([id, ssn])
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
    // "Using an ID that is also a 1!:1 single-field relation"
    #[connector_test(schema(schema_1_1_single_rel))]
    async fn id_also_1_1_single_field_rel(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: { name: "Paul" , age: 40, child: { create: { id: 1, name: "Panther" }}}) {
              name
              age
              child{
                 id
                 name
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"name":"Paul","age":40,"child":{"id":1,"name":"Panther"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(where: { child_id: 1 } data: { age: { set: 41 }}) {
              name
              age
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"name":"Paul","age":41}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(where: { id: 1 } data: { parent: { update: { age: { set: 42 }}}}) {
              parent { age }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"parent":{"age":42}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
              where: { child_id: 2 }
              update: { age: { set: 43 }}
              create: { name: "Milutin", age: 43, child: { create: { id: 2, name: "Nikola" } } }
            ) {
              age
            }
          }"#),
          @r###"{"data":{"upsertOneParent":{"age":43}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteOneParent(
              where: { child_id: 2 }
            ) {
              name
            }
          }"#),
          @r###"{"data":{"deleteOneParent":{"name":"Milutin"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(
              data: {
                name: "Milutin",
                age: 43
                child: {
                  connect: {
                    id: 2
                  }
                }
              }
            ) {
              name
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"name":"Milutin","child":{"id":2}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 3
                name: "Angelina",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":3}}}"###
        );

        // Currently doesn't work
        // insta::assert_snapshot!(
        //   run_query!(&runner, r#"mutation {
        //     updateOneParent(
        //       where: { child: 2 }
        //       data: {
        //         child: {
        //           connect: {
        //             id: 3
        //           }
        //         }
        //       }
        //     ) {
        //       child {
        //         id
        //       }
        //     }
        //   }"#),
        //   @r###""###
        // );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parent: {
                  upsert: {
                    create: {
                      name: "Đuka",
                      age: 40
                    }
                    update: {
                      name: { set: "doesn't matter" }
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
    // "Using an ID that is also a 1!:1 multi-field relation" should "work"
    #[connector_test(schema(schema_1_1_multi_rel), capabilities(CompoundIds))]
    async fn id_also_1_1_multi_field_rel(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: { name: "Paul" , age: 40, child: { create: { id: 1, ssn: "1", name: "Panther" }}}) {
              name
              age
              child{
                 id
                 name
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"name":"Paul","age":40,"child":{"id":1,"name":"Panther"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(where: { child_id_child_ssn: { child_id: 1, child_ssn: "1" }} data: { age: { set: 41 }}) {
              name
              age
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"name":"Paul","age":41}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(where: { id: 1 } data: { parent: { update: { age: { set: 42 }}}}) {
              parent { age }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"parent":{"age":42}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
              where: { child_id_child_ssn: { child_id: 2, child_ssn: "2" } }
              update: { age: { set: 99 }}
              create: { name: "Milutin", age: 43, child: { create: { id: 2, ssn: "2", name: "Nikola" } } }
            ) {
              age
              child {
                id
                ssn
              }
            }
          }"#),
          @r###"{"data":{"upsertOneParent":{"age":43,"child":{"id":2,"ssn":"2"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteOneParent(
              where: { child_id_child_ssn: { child_id: 2, child_ssn: "2" } }
            ) {
              name
            }
          }"#),
          @r###"{"data":{"deleteOneParent":{"name":"Milutin"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(
              data: {
                name: "Milutin",
                age: 43
                child: {
                  connect: {
                    id: 2
                  }
                }
              }
            ) {
              name
              child {
                id
                ssn
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"name":"Milutin","child":{"id":2,"ssn":"2"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 3
                ssn: "3"
                name: "Angelina",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":3}}}"###
        );

        // Currently doesn't work
        // insta::assert_snapshot!(
        //   run_query!(&runner, r#"mutation {
        //     updateOneParent(
        //       where: { child: 2 }
        //       data: {
        //         child: {
        //           connect: {
        //             id: 3
        //           }
        //         }
        //       }
        //     ) {
        //       child {
        //         id
        //       }
        //     }
        //   }"#),
        //   @r###""###
        // );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parent: {
                  upsert: {
                    create: {
                      name: "Đuka",
                      age: 40
                    }
                    update: {
                      name: { set: "doesn't matter" }
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
    // "Using an ID that is also a 1!:M single-field relation" should "work"
    #[connector_test(schema(schema_1_m_single_rel))]
    async fn id_also_1_m_single_field_rel(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: { name: "Paul" , age: 40, child: { create: { id: 1, name: "Panther" }}}) {
              name
              age
              child {
                 id
                 name
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"name":"Paul","age":40,"child":{"id":1,"name":"Panther"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(where: { child_id: 1 } data: { age: { set: 41 }}) {
              name
              age
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"name":"Paul","age":41}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(where: { id: 1 } data: {
              parents: {
                 updateMany: {
                   where: { age: { equals: 41 }}
                   data: { age: { set: 42 } }}
                 }
               }
            ) {
              parents { name age }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"parents":[{"name":"Paul","age":42}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
              where: { child_id: 2 }
              update: { age: { set: 43 }}
              create: { name: "Milutin", age: 43, child: { create: { id: 2, name: "Nikola" } } }
            ) {
              age
            }
          }"#),
          @r###"{"data":{"upsertOneParent":{"age":43}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteOneParent(
              where: { child_id: 2 }
            ) {
              name
            }
          }"#),
          @r###"{"data":{"deleteOneParent":{"name":"Milutin"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(
              data: {
                name: "Milutin",
                age: 43
                child: {
                  connect: {
                    id: 2
                  }
                }
              }
            ) {
              name
              child {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"name":"Milutin","child":{"id":2}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 3
                name: "Angelina",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":3}}}"###
        );

        // Currently doesn't work
        // insta::assert_snapshot!(
        //   run_query!(&runner, r#"mutation {
        //     updateOneParent(
        //       where: { child: 2 }
        //       data: {
        //         child: {
        //           connect: {
        //             id: 3
        //           }
        //         }
        //       }
        //     ) {
        //       child {
        //         id
        //       }
        //     }
        //   }"#),
        //   @r###""###
        // );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parents: {
                  upsert: {
                    where: { child_id: 3 }
                    create: { name: "Đuka", age: 40 }
                    update: { name: { set: "doesn't matter" }}
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
          @r###"{"data":{"updateOneChild":{"id":3,"parents":[{"child":{"id":3}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parents: {
                  updateMany: {
                    where: { age: { equals: 40 }}
                    data: { age: { set: 41 }}
                  }
                }
              }
            ) {
              id
              parents {
                age
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":3,"parents":[{"age":41,"child":{"id":3}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parents: {
                  deleteMany: {
                    age: { equals: 41 }
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
    // "Using an ID that is also a 1!:M multi-field relation" should "work"
    #[connector_test(schema(schema_1_m_multi_rel), capabilities(CompoundIds))]
    async fn id_also_1_m_multi_field_rel(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: { name: "Paul", age: 40, child: { create: { id: 1, ssn: "1", name: "Panther" }}}) {
              name
              age
              child {
                 id
                 name
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"name":"Paul","age":40,"child":{"id":1,"name":"Panther"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(where: { child_id_child_ssn: { child_id: 1, child_ssn: "1" } } data: { age: { set: 41 }}) {
              name
              age
            }
          }"#),
          @r###"{"data":{"updateOneParent":{"name":"Paul","age":41}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(where: { id: 1 } data: {
              parents: {
                 updateMany: {
                   where: { age: { equals: 41 }}
                   data: { age: { set: 42 }}}
                 }
               }
            ) {
              parents { name age }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"parents":[{"name":"Paul","age":42}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneParent(
              where: { child_id_child_ssn: { child_id: 2, child_ssn: "2" } }
              update: { age: { set: 43 }}
              create: { name: "Milutin", age: 43, child: { create: { id: 2, ssn: "2", name: "Nikola" } } }
            ) {
              age
              child {
                id
                ssn
              }
            }
          }"#),
          @r###"{"data":{"upsertOneParent":{"age":43,"child":{"id":2,"ssn":"2"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteOneParent(
              where: { child_id_child_ssn: { child_id: 2, child_ssn: "2" } }
            ) {
              name
            }
          }"#),
          @r###"{"data":{"deleteOneParent":{"name":"Milutin"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(
              data: {
                name: "Milutin",
                age: 43
                child: {
                  connect: {
                    id: 2
                  }
                }
              }
            ) {
              name
              child {
                id
                ssn
              }
            }
          }"#),
          @r###"{"data":{"createOneParent":{"name":"Milutin","child":{"id":2,"ssn":"2"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneChild(
              data: {
                id: 3
                ssn: "3"
                name: "Angelina",
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"createOneChild":{"id":3}}}"###
        );

        // Currently doesn't work
        // insta::assert_snapshot!(
        //   run_query!(&runner, r#"mutation {
        //     updateOneParent(
        //       where: { child: 2 }
        //       data: {
        //         child: {
        //           connect: {
        //             id: 3
        //           }
        //         }
        //       }
        //     ) {
        //       child {
        //         id
        //       }
        //     }
        //   }"#),
        //   @r###""###
        // );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parents: {
                  upsert: {
                    where: { child_id_child_ssn: { child_id: 3, child_ssn: "3" } }
                    create: { name: "Đuka", age: 40 }
                    update: { name: { set: "doesn't matter" }}
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
          @r###"{"data":{"updateOneChild":{"id":3,"parents":[{"child":{"id":3}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3 }
              data: {
                parents: {
                  updateMany: {
                    where: { age: { equals: 40 }}
                    data: { age: { set: 41 }}
                  }
                }
              }
            ) {
              id
              parents {
                age
                child {
                  id
                }
              }
            }
          }"#),
          @r###"{"data":{"updateOneChild":{"id":3,"parents":[{"age":41,"child":{"id":3}}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneChild(
              where: { id: 3}
              data: {
                parents: {
                  deleteMany: {
                    age: { equals: 41 }
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
}
