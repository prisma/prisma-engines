use query_engine_tests::*;

#[test_suite(schema(schema))]
mod order_by_aggr {
    use indoc::indoc;
    use query_engine_tests::{match_connector_result, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name  String
              posts Post[]
              #m2m(categories, Category[], id, Int)
            }

            model Post {
              #id(id, Int, @id)
              title  String
              user   User   @relation(fields: [userId], references: [id])
              userId Int
              #m2m(categories, Category[], id, Int)
            }

            model Category {
              #id(id, Int, @id)
              name   String
              #m2m(posts, Post[], id, Int)
              #m2m(users, User[], id, Int)
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn one2m_count_asc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser(orderBy: { posts: { _count: asc } }) {
              id
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":3,"posts":[]},{"id":1,"posts":[{"title":"alice_post_1"}]},{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser(orderBy: { posts: { _count: desc } }) {
              id
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]},{"id":1,"posts":[{"title":"alice_post_1"}]},{"id":3,"posts":[]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn m2m_count_asc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: { categories: { _count: asc } }) {
              title
              categories(orderBy: { name: asc }) {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_2","categories":[{"name":"Gaming"},{"name":"Hacking"},{"name":"History"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn m2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: { categories: { _count: desc } }) {
              title
              categories(orderBy: { name :asc }) {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"title":"bob_post_2","categories":[{"name":"Gaming"},{"name":"Hacking"},{"name":"History"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_1","categories":[{"name":"Finance"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one2m_count_asc_field_asc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser(orderBy: [{ posts: { _count: asc } }, { name: asc }]) {
              id
              name
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":3,"name":"Motongo","posts":[]},{"id":1,"name":"Alice","posts":[{"title":"alice_post_1"}]},{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"###
        );

        Ok(())
    }

    // "[Combo] Ordering by one2m count asc + field desc" should "work"
    #[connector_test]
    async fn one2m_count_asc_field_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser(orderBy: [{ name: desc }, { posts: { _count: asc } }]) {
              id
              name
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":3,"name":"Motongo","posts":[]},{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]},{"id":1,"name":"Alice","posts":[{"title":"alice_post_1"}]}]}}"###
        );

        Ok(())
    }

    // "[Combo] Ordering by m2m count asc + field desc" should "work"
    #[connector_test]
    async fn m2m_count_asc_field_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ categories: { _count: asc } }, { title: asc }]) {
              title
              categories(orderBy: { name: asc }) {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_2","categories":[{"name":"Gaming"},{"name":"Hacking"},{"name":"History"}]}]}}"###
        );

        Ok(())
    }

    // "[Combo] Ordering by one2m field asc + m2m count desc" should "work"
    #[connector_test]
    async fn one2m_field_asc_m2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ user: { name: asc }}, { categories: { _count: desc }}]) {
              user {
                name
              }
              categories(orderBy: { name: asc }) {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"user":{"name":"Alice"},"categories":[{"name":"News"},{"name":"Society"}]},{"user":{"name":"Bob"},"categories":[{"name":"Gaming"},{"name":"Hacking"},{"name":"History"}]},{"user":{"name":"Bob"},"categories":[{"name":"Finance"}]}]}}"###
        );

        Ok(())
    }

    // "[2+ Hops] Ordering by m2one2m count asc" should "work"
    #[connector_test]
    async fn m2one2m_count_asc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ user: { categories: { _count: asc } } }, { id: asc }]) {
              id
              user { categories { name } }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":1,"user":{"categories":[{"name":"Startup"}]}},{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"###
        );

        Ok(())
    }

    // "[2+ Hops] Ordering by m2one2m count desc" should "work"
    #[connector_test]
    async fn m2one2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match_connector_result!(
            &runner,
            r#"{
              findManyPost(orderBy: { user: { categories: { _count: desc } } }) {
                id
                user { categories { name } }
              }
            }"#,
            _ => vec![
              r#"{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"user":{"categories":[{"name":"Startup"}]}}]}}"#,
              r#"{"data":{"findManyPost":[{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"user":{"categories":[{"name":"Startup"}]}}]}}"#
            ]
        );
        Ok(())
    }

    // "[Combo][2+ Hops] Ordering by m2m count asc + m2one2m count desc" should "work"
    #[connector_test]
    async fn m2m_count_asc_m2one2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ categories: { _count: asc }}, { user: { categories: { _count: desc }} }]) {
              id
              categories(orderBy: { name: asc }) {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"categories":[{"name":"Finance"}]},{"id":1,"categories":[{"name":"News"},{"name":"Society"}]},{"id":3,"categories":[{"name":"Gaming"},{"name":"Hacking"},{"name":"History"}]}]}}"###
        );

        Ok(())
    }

    // "[Combo][2+ Hops] Ordering by m2one field asc + m2one2m count desc" should "work"
    #[connector_test]
    async fn m2one_field_asc_m2one2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match_connector_result!(
            &runner,
            r#"{
              findManyPost(orderBy: [{ user: { name: asc }}, { user: { categories: { _count: desc }} }]) {
                id
                user {
                  name
                  categories { name }
                }
              }
            }"#,
            Postgres(_) | MySql(_) => vec![
              r#"{"data":{"findManyPost":[{"id":1,"user":{"name":"Alice","categories":[{"name":"Startup"}]}},{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"#,
              r#"{"data":{"findManyPost":[{"id":1,"user":{"name":"Alice","categories":[{"name":"Startup"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"#
            ],
            _ => vec![r#"{"data":{"findManyPost":[{"id":1,"user":{"name":"Alice","categories":[{"name":"Startup"}]}},{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"#]
        );

        Ok(())
    }

    fn m2one2one2m() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
              b_id Int?
              b    B?   @relation(fields: [b_id], references: [id])
            }
            
            model B {
              #id(id, Int, @id)
              as   A[]
              c_id Int?
              c    C?   @relation(fields: [c_id], references: [id])
            }
            
            model C {
              #id(id, Int, @id)
              bs B[]
              ds D[]
            }
            
            model D {
              #id(id, Int, @id)
              c_id Int?
              c    C?   @relation(fields: [c_id], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    // "[3+ Hops] Ordering by m2one2one2one2m count desc" should "work"
    #[connector_test(schema(m2one2one2m))]
    async fn m2one2one2m_count_asc(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                    createOneA(data: {
                      id: 1,
                      b: {
                        create: {
                          id: 1,
                          c: {
                            create: {
                              id: 1,
                              ds: {
                                create: [{ id: 1 }]
                              }
                            }
                          }
                        }
                      }
                    }) {
                      id
                    }
              }"#
        );
        run_query!(
            &runner,
            r#"mutation {
                    createOneA(data: {
                      id: 2,
                      b: {
                        create: {
                          id: 2,
                          c: {
                            create: {
                              id: 2,
                              ds: {
                                create: [{ id: 2 }, { id: 3 }]
                              }
                            }
                          }
                        }
                      }
                    }) {
                      id
                    }
                }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyA(orderBy: { b: { c: { ds: { _count: asc } } } }) {
                  id
                }
              }
              "#),
          @r###"{"data":{"findManyA":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    // "[3+ Hops] Ordering by m2one2one2one2m count desc" should "work"
    #[connector_test(schema(m2one2one2m))]
    async fn m2one2one2m_count_desc(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneA(data: {
                  id: 1,
                  b: {
                    create: {
                      id: 1,
                      c: {
                        create: {
                          id: 1,
                          ds: {
                            create: [{ id: 1 }]
                          }
                        }
                      }
                    }
                  }
                }) {
                  id
                }
          }"#
        );
        run_query!(
            &runner,
            r#"mutation {
                createOneA(data: {
                  id: 2,
                  b: {
                    create: {
                      id: 2,
                      c: {
                        create: {
                          id: 2,
                          ds: {
                            create: [{ id: 2 }, { id: 3 }]
                          }
                        }
                      }
                    }
                  }
                }) {
                  id
                }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyA(orderBy: { b: { c: { ds: { _count: desc } } } }) {
              id
            }
          }
          "#),
          @r###"{"data":{"findManyA":[{"id":2},{"id":1}]}}"###
        );

        Ok(())
    }

    // With pagination tests

    // "[Cursor] Ordering by one2m count asc" should "work"
    #[connector_test]
    async fn cursor_one2m_count_asc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser(orderBy: { posts: { _count: asc } }, cursor: { id: 1 }) {
              id
              posts {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":1,"posts":[{"id":1}]},{"id":2,"posts":[{"id":2},{"id":3}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor] Ordering by one2m count desc" should "work"
    #[connector_test]
    async fn cursor_one2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser(orderBy: { posts: { _count: desc } }, cursor: { id: 1 }) {
              id
              posts {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":1,"posts":[{"id":1}]},{"id":3,"posts":[]}]}}"###
        );

        Ok(())
    }

    // "[Cursor] Ordering by m2m count asc" should "work"
    #[connector_test]
    async fn cursor_m2m_count_asc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: { categories: { _count: asc } }, cursor: { id: 2 }, take: 2) {
              id
              title
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]},{"id":1,"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor] Ordering by m2m count desc" should "work"
    #[connector_test]
    async fn cursor_m2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: { categories: { _count: desc } }, cursor: { id: 1 }, take: 2) {
              id
              title
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":1,"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo] Ordering by one2m count asc + field asc"
    #[connector_test]
    async fn cursor_one2m_count_asc_field_asc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser(orderBy: [{ posts: { _count: asc } }, { name: asc }], cursor: { id: 2 }) {
              id
              name
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo] Ordering by one2m count asc + field desc" should "work"
    #[connector_test]
    async fn cursor_one2m_count_asc_field_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyUser(orderBy: [{ name: desc }, { posts: { _count: asc } }], cursor: { id: 2 }, take: 1) {
              id
              name
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo] Ordering by m2m count asc + field desc" should "work"
    #[connector_test]
    async fn cursor_m2m_count_asc_field_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ categories: { _count: asc } }, { title: asc }], cursor: { id: 2 }, take: 2) {
              id
              title
              categories(orderBy: { name: asc }) {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]},{"id":1,"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo] Ordering by one2m field asc + m2m count desc" should "work"
    #[connector_test]
    async fn cursor_one2m_field_asc_m2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ user: { name: asc }}, { categories: { _count: desc }}], cursor: { id: 1 }, take: 2) {
              id
              title
              user {
                name
              }
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":1,"title":"alice_post_1","user":{"name":"Alice"},"categories":[{"name":"News"},{"name":"Society"}]},{"id":3,"title":"bob_post_2","user":{"name":"Bob"},"categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][2+ Hops] Ordering by m2one2m count asc" should "work"
    #[connector_test]
    async fn cursor_m2one2m_count_asc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ user: { categories: { _count: asc } } }, { id: asc }], cursor: { id: 2 }, take: 1) {
              id
              user { categories { name } }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"###
        );

        Ok(())
    }

    // "[Cursor][2+ Hops] Ordering by m2one2m count desc" should "work"
    #[connector_test]
    async fn cursor_m2one2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ user: { categories: { _count: desc } } }, { id: asc }], cursor: { id: 2 }, take: 2) {
              id
              user { categories { name } }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo][2+ Hops] Ordering by m2m count asc + m2one2m count desc" should "work"
    #[connector_test]
    async fn cursor_m2m_count_asc_m2one2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ categories: { _count: asc }}, { user: { categories: { _count: desc }} }], cursor: { id: 2 }, take: 2) {
              id
              categories { name }
              user { categories { name } }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"categories":[{"name":"Finance"}],"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"categories":[{"name":"News"},{"name":"Society"}],"user":{"categories":[{"name":"Startup"}]}}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo][2+ Hops] Ordering by m2one field asc + m2one2m count desc" should "work"
    #[connector_test]
    async fn cursor_m2one_field_asc_m2one2m_count_desc(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(orderBy: [{ user: { name: asc }}, { user: { categories: { _count: desc }} }, { id: asc }], cursor: { id: 2 }, take: 2) {
              id
              user {
                name
                categories { name }
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"###
        );

        Ok(())
    }

    // "[Cursor][3+ Hops] Ordering by m2one2one2one2m count desc" should "work"
    #[connector_test(schema(m2one2one2m))]
    async fn cursor_m2one2one2m_count_desc(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                    createOneA(data: {
                      id: 1,
                      b: {
                        create: {
                          id: 1,
                          c: {
                            create: {
                              id: 1,
                              ds: {
                                create: [{ id: 1 }]
                              }
                            }
                          }
                        }
                      }
                    }) {
                      id
                    }
              }"#
        );
        run_query!(
            &runner,
            r#"mutation {
                    createOneA(data: {
                      id: 2,
                      b: {
                        create: {
                          id: 2,
                          c: {
                            create: {
                              id: 2,
                              ds: {
                                create: [{ id: 2 }, { id: 3 }]
                              }
                            }
                          }
                        }
                      }
                    }) {
                      id
                    }
                }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyA(
                  orderBy: { b: { c: { ds: { _count: desc } } } },
                  cursor: { id: 1 },
                  take: 1
                ) {
                  id
                }
              }
              "#),
          @r###"{"data":{"findManyA":[{"id":1}]}}"###
        );

        Ok(())
    }

    // https://github.com/prisma/prisma/issues/8036
    fn schema_regression_8036() -> String {
        let schema = indoc! {
            r#"model Post {
              #id(id, Int, @id)
              title       String
              #m2m(LikedPeople, Person[], id, Int)
            }

            model Person {
              #id(id, Int, @id)
              name      String
              #m2m(likePosts, Post[], id, Int)
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_regression_8036))]
    async fn count_m2m_records_not_connected(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOnePerson(data: { id: 1, name: "Alice" }) { id } }"#
        );
        run_query!(
            runner,
            r#"mutation { createOnePost(data: { id: 1, title: "First", LikedPeople: { connect: { id: 1 } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"mutation { createOnePost(data: { id: 2, title: "Second" }) { id } }"#
        );
        run_query!(
            runner,
            r#"mutation { createOnePost(data: { id: 3, title: "Third" }) { id } }"#
        );
        run_query!(
            runner,
            r#"mutation { createOnePost(data: { id: 4, title: "Fourth" }) { id } }"#
        );
        run_query!(
            runner,
            r#"mutation { createOnePost(data: { id: 5, title: "Fifth" }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(
              cursor: { id: 1 },
              skip: 1,
              take: 4
              orderBy: [{ LikedPeople: { _count: desc } }, { id: asc }]
            ) {
              id
              title
              _count {
                LikedPeople
              }
            }
          }
          "#),
          @r###"{"data":{"findManyPost":[{"id":2,"title":"Second","_count":{"LikedPeople":0}},{"id":3,"title":"Third","_count":{"LikedPeople":0}},{"id":4,"title":"Fourth","_count":{"LikedPeople":0}},{"id":5,"title":"Fifth","_count":{"LikedPeople":0}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyPost(
              cursor: { id: 1 }
              take: 2
              orderBy: [{ title: asc }, { LikedPeople: { _count: asc } }, { id: asc }]
            ) {
              id
              _count {
                LikedPeople
              }
            }
          }
          "#),
          @r###"{"data":{"findManyPost":[{"id":1,"_count":{"LikedPeople":1}},{"id":4,"_count":{"LikedPeople":0}}]}}"###
        );

        Ok(())
    }

    fn nested_one2m_schema() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
            
              bs B[]
            }
            
            model B {
              #id(id, Int, @id)
            
              A   A?   @relation(fields: [aId], references: [id])
              aId Int?
              
              cId Int?
              c   C?   @relation(fields: [cId], references: [id])
            }
            
            model C {
              #id(id, Int, @id)
              B    B[]
            
              dId Int?
              d   D?   @relation(fields: [dId], references: [id])
            }
            
            model D {
              #id(id, Int, @id)
              C    C[]
            
              es E[]
            }
            
            model E {
              #id(id, Int, @id)
            
              dId Int?
              D   D?   @relation(fields: [dId], references: [id])
            }
            
            "#
        };

        schema.to_owned()
    }

    // [Nested 2+ Hops] Ordering by one2one2m count should "work
    #[connector_test(schema(nested_one2m_schema))]
    async fn nested_one2m_count(runner: Runner) -> TestResult<()> {
        // test data
        run_query!(
            &runner,
            r#"mutation {
            createOneA(
              data: {
                id: 1,
                bs: {
                  create: [
                    {
                      id: 1,
                      c: {
                        create: {
                          id: 1,
                          d: { create: { id: 1, es: { create: [{ id: 1 }] } } }
                        }
                      }
                    }
                    {
                      id: 2,
                      c: {
                        create: {
                          id: 2,
                          d: { create: { id: 2, es: { create: [{ id: 2 }, { id: 3 }] } } }
                        }
                      }
                    }
                  ]
                }
              }
            ) {
              id
            }
          }
          "#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyA {
              id
              bs(orderBy: { c: { d: { es: { _count: asc } } } }) {
                id
                c {
                  d {
                    es {
                      id
                    }
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":1,"c":{"d":{"es":[{"id":1}]}}},{"id":2,"c":{"d":{"es":[{"id":2},{"id":3}]}}}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyA {
              id
              bs(orderBy: { c: { d: { es: { _count: desc } } } }) {
                id
                c {
                  d {
                    es {
                      id
                    }
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":2,"c":{"d":{"es":[{"id":2},{"id":3}]}}},{"id":1,"c":{"d":{"es":[{"id":1}]}}}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyA {
              id
              bs(orderBy: { c: { d: { id: asc } } }) {
                id
                c {
                  d {
                    id
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":1,"c":{"d":{"id":1}}},{"id":2,"c":{"d":{"id":2}}}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyA {
              id
              bs(orderBy: { c: { d: { id: desc } } }) {
                id
                c {
                  d {
                    id
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":2,"c":{"d":{"id":2}}},{"id":1,"c":{"d":{"id":1}}}]}]}}"###
        );

        Ok(())
    }

    fn nested_m2m_schema() -> String {
        let schema = indoc! {
            r#"model A {
            #id(id, Int, @id)
          
            #m2m(bs, B[], id, Int)
          }
          
          model B {
            #id(id, Int, @id)
          
            #m2m(as, A[], id, Int)
            
            cId Int?
            c   C?   @relation(fields: [cId], references: [id])
          }
          
          model C {
            #id(id, Int, @id)
            B    B[]
          
            dId Int?
            d   D?   @relation(fields: [dId], references: [id])
          }
          
          model D {
            #id(id, Int, @id)
            C    C[]
          
            es E[]
            #m2m(fs, F[], id, Int)
          }
          
          model E {
            #id(id, Int, @id)
          
            dId Int?
            D   D?   @relation(fields: [dId], references: [id])
          }

          model F {
            #id(id, Int, @id)

            #m2m(ds, D[], id, Int)
          }
          
          "#
        };

        schema.to_owned()
    }

    // [Nested 2+ Hops] Ordering by m2one2one2m count should "work
    // Regression test for https://github.com/prisma/prisma/issues/22926
    #[connector_test(schema(nested_m2m_schema))]
    async fn nested_m2m_count(runner: Runner) -> TestResult<()> {
        // test data
        run_query!(
            &runner,
            r#"mutation {
        createOneA(
          data: {
            id: 1,
            bs: {
              create: [
                {
                  id: 1,
                  c: {
                    create: {
                      id: 1,
                      d: {
                        create: {
                          id: 1,
                          es: { create: [{ id: 1 }] },
                          fs: { create: [{ id: 1 }] }
                        }
                      }
                    }
                  }
                }
                {
                  id: 2,
                  c: {
                    create: {
                      id: 2,
                      d: {
                        create: {
                          id: 2,
                          es: { create: [{ id: 2 }, { id: 3 }] }
                          fs: { create: [{ id: 2 }, { id: 3 }] }
                        }
                      }
                    }
                  }
                }
              ]
            }
          }
        ) {
          id
        }
      }
      "#
        );

        // count asc on 1-m
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findManyA {
            id
            bs(orderBy: { c: { d: { es: { _count: asc } } } }) {
              id
              c {
                d {
                  es {
                    id
                  }
                }
              }
            }
          }
        }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":1,"c":{"d":{"es":[{"id":1}]}}},{"id":2,"c":{"d":{"es":[{"id":2},{"id":3}]}}}]}]}}"###
        );

        // count desc on 1-m
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findManyA {
            id
            bs(orderBy: { c: { d: { es: { _count: desc } } } }) {
              id
              c {
                d {
                  es {
                    id
                  }
                }
              }
            }
          }
        }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":2,"c":{"d":{"es":[{"id":2},{"id":3}]}}},{"id":1,"c":{"d":{"es":[{"id":1}]}}}]}]}}"###
        );

        // count asc on m-n
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findManyA {
            id
            bs(orderBy: { c: { d: { fs: { _count: asc } } } }) {
              id
              c {
                d {
                  fs {
                    id
                  }
                }
              }
            }
          }
        }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":1,"c":{"d":{"fs":[{"id":1}]}}},{"id":2,"c":{"d":{"fs":[{"id":2},{"id":3}]}}}]}]}}"###
        );

        // count desc on m-n
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findManyA {
            id
            bs(orderBy: { c: { d: { fs: { _count: desc } } } }) {
              id
              c {
                d {
                  fs {
                    id
                  }
                }
              }
            }
          }
        }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":2,"c":{"d":{"fs":[{"id":2},{"id":3}]}}},{"id":1,"c":{"d":{"fs":[{"id":1}]}}}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findManyA {
            id
            bs(orderBy: { c: { d: { id: asc } } }) {
              id
              c {
                d {
                  id
                }
              }
            }
          }
        }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":1,"c":{"d":{"id":1}}},{"id":2,"c":{"d":{"id":2}}}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findManyA {
            id
            bs(orderBy: { c: { d: { id: desc } } }) {
              id
              c {
                d {
                  id
                }
              }
            }
          }
        }"#),
          @r###"{"data":{"findManyA":[{"id":1,"bs":[{"id":2,"c":{"d":{"id":2}}},{"id":1,"c":{"d":{"id":1}}}]}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, name: "Alice", categories: { create: [{ id: 1, name: "Startup" }] }, posts: { create: { id: 1, title: "alice_post_1", categories: { create: [{ id: 2, name: "News" }, { id: 3, name: "Society" }] }} } }"#).await?;
        create_row(runner, r#"{ id: 2, name: "Bob", categories: { create: [{ id: 4, name: "Computer Science" }, { id: 5, name: "Music" }] }, posts: { create: [{ id: 2, title: "bob_post_1", categories: { create: [{ id: 6, name: "Finance" }] } }, { id: 3, title: "bob_post_2", categories: { create: [{ id: 7, name: "History" }, { id: 8, name: "Gaming" }, { id: 9, name: "Hacking" }] } }] } }"#).await?;
        create_row(runner, r#"{ id: 3, name: "Motongo" }"#).await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneUser(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
