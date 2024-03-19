use query_engine_tests::*;

// Note: Except for m:n cases that are always resolved using the primary identifier of the models, we use different
// relation links to ensure that the underlying QE logic correctly uses link resolvers instead of
// only primary id resolvers.
#[test_suite(exclude(Sqlite("cfd1")))]
mod connect_or_create {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, String, @id, @default(cuid()))
              #m2m(manyB, ModelB[], id, String)
            }

            model ModelB {
              #id(id, String, @id, @default(cuid()))
              #m2m(manyA, ModelA[], id, String)
            }"#
        };

        schema.to_owned()
    }

    // "A m:n relation connectOrCreate" should "always work"
    #[connector_test(schema(schema_1))]
    async fn m2n_connect_or_create(runner: Runner) -> TestResult<()> {
        // Both records are new
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{
            createOneModelA(data: {
              id: "A1"
              manyB: {
                connectOrCreate: {
                  where: { id: "B1" }
                  create: {
                    id: "B1"
                  }
                }
              }
            }) {
              id
              manyB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A1","manyB":[{"id":"B1"}]}}}"###
        );

        // New parent, connect existing child
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{
            createOneModelA(data: {
              id: "A2"
              manyB: {
                connectOrCreate: {
                  where: { id: "B1" }
                  create: {
                    id: "Doesn't matter"
                  }
                }
              }
            }) {
              id
              manyB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A2","manyB":[{"id":"B1"}]}}}"###
        );

        // Update a parent to connect 2 new children
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelA(
              where: { id: "A1" }
              data: {
                manyB: {
                  connectOrCreate: [{
                    where: { id: "B2" }
                    create: {
                      id: "B2"
                    }
                  },{
                    where: { id: "B3" }
                    create: {
                      id: "B3"
                    }
                  }]
                }
              }
            ) {
              id
              manyB {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneModelA":{"id":"A1","manyB":[{"id":"B1"},{"id":"B2"},{"id":"B3"}]}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, String, @id, @default(cuid()))
              b_u String

              oneB ModelB @relation(fields: [b_u], references: [b_u])
            }

            model ModelB {
              #id(id, String, @id, @default(cuid()))
              b_u String @unique

              manyA ModelA[]
            }"#
        };

        schema.to_owned()
    }

    // "A 1!:m relation connectOrCreate" should "work"
    #[connector_test(schema(schema_2))]
    async fn one_req_2m_connect_or_create(runner: Runner) -> TestResult<()> {
        // Inlined in parent cases
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: "A1"
              oneB: {
                connectOrCreate: {
                  where: { b_u: "B1" }
                  create: {
                    id: "B_id_1",
                    b_u: "B1"
                  }
                }
              }
            }) {
              id
              oneB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A1","oneB":{"id":"B_id_1"}}}}"###
        );

        // Create new parent, connect to existing child
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: "A2"
              oneB: {
                connectOrCreate: {
                  where: { b_u: "B1" }
                  create: {
                    id: "B_id_1",
                    b_u: "B1"
                  }
                }
              }
            }) {
              id
              oneB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A2","oneB":{"id":"B_id_1"}}}}"###
        );

        // Inlined in child cases
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelB(
              where: { b_u: "B1" }
              data: {
                manyA: {
                  connectOrCreate: [{
                    where: { id: "A3" }
                    create: {
                      id: "A3"
                    }
                  },{
                    where: { id: "A4" }
                    create: {
                      id: "A4"
                    }
                  }]
                }
              }
            ) {
              id
              manyA {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneModelB":{"id":"B_id_1","manyA":[{"id":"A1"},{"id":"A2"},{"id":"A3"},{"id":"A4"}]}}}"###
        );

        // Create new child, connect existing parent (disconnects parent from B1)
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelB(
              data: {
                id: "B_id_2"
                b_u: "B2",
                manyA: {
                  connectOrCreate: {
                    where: { id: "A1" }
                    create: {
                      id: "A1"
                    }
                  }
                }
              }
            ) {
              id
              manyA {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelB":{"id":"B_id_2","manyA":[{"id":"A1"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueModelA(where: { id: "A1" }) {
              oneB {
                b_u
              }
            }
          }"#),
          @r###"{"data":{"findUniqueModelA":{"oneB":{"b_u":"B2"}}}}"###
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, String, @id, @default(cuid()))
              b_u String?

              oneB ModelB? @relation(fields: [b_u], references: [b_u])
            }

            model ModelB {
              #id(id, String, @id, @default(cuid()))
              b_u String @unique

              manyA ModelA[]
            }"#
        };

        schema.to_owned()
    }

    // "A 1:m relation connectOrCreate" should "work"
    #[connector_test(schema(schema_3))]
    async fn one2m_connect_or_create(runner: Runner) -> TestResult<()> {
        // Inlined in parent cases

        // Both records are new
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: "A1"
              oneB: {
                connectOrCreate: {
                  where: { b_u: "B1" }
                  create: {
                    id: "B_id_1",
                    b_u: "B1"
                  }
                }
              }
            }) {
              id
              oneB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A1","oneB":{"id":"B_id_1"}}}}"###
        );

        // Create new parent, connect to existing child
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: "A2"
              oneB: {
                connectOrCreate: {
                  where: { b_u: "B1" }
                  create: {
                    id: "B_id_1",
                    b_u: "B1"
                  }
                }
              }
            }) {
              id
              oneB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A2","oneB":{"id":"B_id_1"}}}}"###
        );

        // Inlined in child cases

        // Connect 2 more children (ModelAs here)
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModelB(
              where: { b_u: "B1" }
              data: {
                manyA: {
                  connectOrCreate: [{
                    where: { id: "A3" }
                    create: {
                      id: "A3"
                    }
                  },{
                    where: { id: "A4" }
                    create: {
                      id: "A4"
                    }
                  }]
                }
              }
            ) {
              id
              manyA {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneModelB":{"id":"B_id_1","manyA":[{"id":"A1"},{"id":"A2"},{"id":"A3"},{"id":"A4"}]}}}"###
        );

        // Create new child, connect existing parent (disconnects parent from B1)
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelB(
              data: {
                id: "B_id_2"
                b_u: "B2",
                manyA: {
                  connectOrCreate: {
                    where: { id: "A1" }
                    create: {
                      id: "A1"
                    }
                  }
                }
              }
            ) {
              id
              manyA {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelB":{"id":"B_id_2","manyA":[{"id":"A1"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueModelA(where: { id: "A1" }) {
              oneB {
                b_u
              }
            }
          }"#),
          @r###"{"data":{"findUniqueModelA":{"oneB":{"b_u":"B2"}}}}"###
        );

        Ok(())
    }

    fn schema_4() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, String, @id)
              fieldA String?
              A2B    A2B[]   @relation("A2_A2B")
            }

            model B {
              #id(id, String, @id)
              fieldB String
              A2B    A2B[]  @relation("B2_A2B")
            }

            model A2B {
              a_id    String
              b_id    String
              fieldAB Int
              a       A      @relation("A2_A2B", fields: [a_id], references: [id])
              b       B      @relation("B2_A2B", fields: [b_id], references: [id])

              @@id([a_id, b_id])
              @@index([b_id], name: "fk_b")
              @@map("_A2B")
            }"#
        };

        schema.to_owned()
    }

    // Regression test for failing internal graph transformations.
    // "Query reordering" should "not break connectOrCreate"
    // TODO(dom): Not working for mongo
    #[connector_test(schema(schema_4), capabilities(CompoundIds))]
    async fn query_reordering_works(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {upsertOneA2B(
            where: {
              a_id_b_id: {
                a_id: "a"
                b_id: "b"
              }
            },
            create: {
              a: {
                connectOrCreate: {
                  where:  { id: "a" }
                  create: { id: "a", fieldA: "Field A" }
                }
              }
              b: {
                connectOrCreate: {
                  where:  { id: "b" }
                  create: { id: "b", fieldB: "Field B" }
                }
              }
              fieldAB: 1
            }
            update: {
              fieldAB: 1
            }) {
              fieldAB
            }
          }"#),
          @r###"{"data":{"upsertOneA2B":{"fieldAB":1}}}"###
        );

        Ok(())
    }

    fn schema_5() -> String {
        let schema = indoc! {
            r#"model Parent {
            #id(id, Int, @id)
          
            child Child?
          }
          
          model Child {
            #id(id, Int, @id)

            parentId Int?   @unique          
            parent Parent? @relation(fields: [parentId], references: [id])
          }
          "#
        };

        schema.to_owned()
    }

    // Regression test for https://github.com/prisma/prisma/issues/16090
    #[connector_test(schema(schema_5))]
    async fn one2one_update_if_no_child_connected_yet(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneParent(data: { id: 1 }) { id } }"#);
        run_query!(&runner, r#"mutation { createOneChild(data: { id: 1 }) { id } }"#);

        // connect (without child connected yet)
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: {
            child: {
              connectOrCreate: { where: { id: 1 }, create: { id: 1 } }
            }
          }) {
            id
            child { id }
          } }"#),
          @r###"{"data":{"updateOneParent":{"id":1,"child":{"id":1}}}}"###
        );

        // connect (child already connected)
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: {
            child: {
              connectOrCreate: { where: { id: 1 }, create: { id: 1 } }
            }
          }) {
            id
            child { id }
          } }"#),
          @r###"{"data":{"updateOneParent":{"id":1,"child":{"id":1}}}}"###
        );

        Ok(())
    }

    // Regression test for https://github.com/prisma/prisma/issues/16090
    #[connector_test(schema(schema_5))]
    async fn one2one_upsert_if_no_child_connected_yet(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneParent(data: { id: 1 }) { id } }"#);
        run_query!(&runner, r#"mutation { createOneChild(data: { id: 1 }) { id } }"#);

        // connect (without child connected yet)
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { upsertOneParent(
            where: { id: 1 },
            create: { id: 1 },
            update: {
              child: {
                connectOrCreate: { where: { id: 1 }, create: { id: 1 } }
              }
            }
          ) {
            id
            child { id }
          } }"#),
          @r###"{"data":{"upsertOneParent":{"id":1,"child":{"id":1}}}}"###
        );

        // connect (child already connected)
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { upsertOneParent(
            where: { id: 1 },
            create: { id: 1 },
            update: {
              child: {
                connectOrCreate: { where: { id: 1 }, create: { id: 1 } }
              }
            }
          ) {
            id
            child { id }
          } }"#),
          @r###"{"data":{"upsertOneParent":{"id":1,"child":{"id":1}}}}"###
        );

        Ok(())
    }
}
