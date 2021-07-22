use query_engine_tests::*;

#[test_suite]
mod multi_field_uniq_mut {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};

    fn schema_one2one() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name String

              blog Blog?
            }

            model Blog {
              #id(id, Int, @id)
              title     String
              category  String
              author_id Int?

              author   User? @relation(fields: [author_id], references: [id])
              @@unique([title, category])
            }"#
        };

        schema.to_owned()
    }

    // CONNECTS //

    // "A nested connect on a one-to-one relation with a multi-field unique" should "work"
    #[connector_test(schema(schema_one2one))]
    async fn nested_connect_one2one_rel(runner: Runner) -> TestResult<()> {
        create_user(&runner, r#"{ id: 1, name: "Thomas the Tank Engine" }"#).await?;
        create_blog(
            &runner,
            r#"{ id: 1, title: "Thomas has seen it all. Thomas is leaving." category: "Horror" }"#,
        )
        .await?;

        let res = run_query_json!(
            &runner,
            r#"mutation {
                updateOneUser(where: {
                  id: 1
                }
                data: {
                  blog: {
                    connect: {
                      title_category: {
                        title: "Thomas has seen it all. Thomas is leaving."
                        category: "Horror"
                      }
                    }
                }}){
                  blog {
                    id
                  }
                }
            }"#
        );
        let blog_id = &res["data"]["updateOneUser"]["blog"]["id"].to_string();

        assert_eq!(blog_id, "1");

        Ok(())
    }

    fn schema_one2m() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name  String

              blogs Blog[]
            }

            model Blog {
              #id(id, Int, @id)
              title    String
              category String
              author_id Int?

              author   User? @relation(fields: [author_id], references: [id])
              @@unique([title, category])
            }"#
        };

        schema.to_owned()
    }

    //"A nested connect on a one-to-many relation with a multi-field unique" should "work"
    #[connector_test(schema(schema_one2m))]
    async fn nested_connect_one2m_rel(runner: Runner) -> TestResult<()> {
        create_user(&runner, r#"{ id: 1, name: "Thomas the Tank Engine" }"#).await?;
        create_blog(
            &runner,
            r#"{ id: 1, title: "Thomas has seen it all. Thomas is leaving." category: "Horror" }"#,
        )
        .await?;

        let res = run_query_json!(
            &runner,
            r#"mutation {
          updateOneUser(where: {
            id: 1
          }
          data: {
            blogs: {
              connect: {
                title_category: {
                  title: "Thomas has seen it all. Thomas is leaving."
                  category: "Horror"
                }
              }
          }}){
            blogs {
              id
            }
          }
        }"#
        );
        let blogs = &res["data"]["updateOneUser"]["blogs"];
        let blogs_len = match blogs {
            serde_json::Value::Array(array) => array.len(),
            _ => unreachable!(),
        };

        assert_eq!(blogs_len, 1);

        Ok(())
    }

    // DISCONNECTS

    // "A nested disconnect with a multi-field unique" should "work"
    #[connector_test(schema(schema_one2m))]
    async fn nested_disconnect_multi_field_uniq(runner: Runner) -> TestResult<()> {
        create_user(
            &runner,
            r#"{
              id: 1,
              name: "Sly Marbo"
              blogs: {
                create: [{
                  id: 1,
                  title: "AAAAAAAAAAA!"
                  category: "Drama"
                },
                {
                  id: 2,
                  title: "The Secret of AAAAAAAAAAA!"
                  category: "Drama"
                }]
              }
          }"#,
        )
        .await?;

        let res = run_query_json!(
            &runner,
            r#"mutation {
                updateOneUser(where: {
                  id: 1
                }
                data: {
                  blogs: {
                    disconnect: {
                      title_category: {
                        title: "AAAAAAAAAAA!"
                        category: "Drama"
                      }
                    }
                }}) {
                  blogs {
                    id
                  }
                }
            }"#
        );
        let blogs = &res["data"]["updateOneUser"]["blogs"];
        let blogs_len = match blogs {
            serde_json::Value::Array(array) => array.len(),
            _ => unreachable!(),
        };

        assert_eq!(blogs_len, 1);

        Ok(())
    }

    // UPDATES

    fn schema_multi_uniq() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              first_name  String
              last_name   String

              @@unique([first_name, last_name])
            }"#
        };

        schema.to_owned()
    }

    // "An update with a multi-field unique" should "work"
    #[connector_test(schema(schema_multi_uniq))]
    async fn update_multi_field_uniq(runner: Runner) -> TestResult<()> {
        create_user(&runner, r#"{ id: 1, first_name: "Justin" last_name: "Case" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneUser(where: {
              first_name_last_name: {
                first_name: "Justin"
                last_name: "Case"
              }
            }
            data: {
              first_name: { set: "Worst" }
            }) {
              first_name
            }
          }"#),
          @r###"{"data":{"updateOneUser":{"first_name":"Worst"}}}"###
        );

        Ok(())
    }

    fn schema_nested_multi_uniq() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name  String
              blogs Blog[]
            }

            model Blog {
              #id(id, Int, @id)
              title     String
              category  String
              published Boolean
              author_id Int?

              author   User? @relation(fields: [author_id], references: [id])
              @@unique([title, category])
            }"#
        };

        schema.to_owned()
    }

    // "A nested update with a multi-field unique" should "work"
    #[connector_test(schema(schema_nested_multi_uniq))]
    async fn nested_update_multi_uniq_field(runner: Runner) -> TestResult<()> {
        create_user(
            &runner,
            r#"{
                  id: 1
                  name: "King Arthur"
                  blogs: {
                    create: [{
                      id: 1,
                      title: "A Practical Guide to the Monster of Caerbannog"
                      category: "Education"
                      published: false
                    }]
                  }
              }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneUser(where: {
              id: 1
            }
            data: {
              blogs: {
                update: {
                  where: {
                    title_category: {
                      title: "A Practical Guide to the Monster of Caerbannog"
                      category: "Education"
                    }
                  },
                  data: {
                    published: { set: true }
                  }
                }
            }}) {
              blogs {
                published
              }
            }
          }"#),
          @r###"{"data":{"updateOneUser":{"blogs":[{"published":true}]}}}"###
        );

        Ok(())
    }

    // DELETES

    //
    #[connector_test(schema(schema_multi_uniq))]
    async fn delete_multi_field_uniq(runner: Runner) -> TestResult<()> {
        create_user(&runner, r#"{ id: 1, first_name: "Darth", last_name: "Llama" }"#).await?;

        let res = run_query_json!(
            &runner,
            r#"mutation {
          deleteOneUser(where: {
            first_name_last_name: {
              first_name: "Darth"
              last_name: "Llama"
            }
          }) {
            id
          }
        }"#
        );
        let user_id = &res["data"]["deleteOneUser"]["id"].to_string();

        assert_eq!(user_id, "1");

        Ok(())
    }

    // "A nested delete with a multi-field unique" should "work"
    #[connector_test(schema(schema_one2m))]
    async fn nested_delete_multi_field_uniq(runner: Runner) -> TestResult<()> {
        create_user(
            &runner,
            r#"{
                  id: 1,
                  name: "Matt Eagle"
                  blogs: {
                    create: [{
                      id: 1,
                      title: "The Perfect German 'Mettigel'"
                      category: "Cooking"
                    }]
                  }
              }"#,
        )
        .await?;

        let res = run_query_json!(
            &runner,
            r#"mutation {
          updateOneUser(where: {
            id: 1
          }
          data: {
            blogs: {
              delete: {
                  title_category: {
                    title: "The Perfect German 'Mettigel'"
                    category: "Cooking"
                  }
              }
          }}) {
            blogs {
              id
            }
          }
        }"#
        );
        let blogs = &res["data"]["updateOneUser"]["blogs"];
        let blogs_len = match blogs {
            serde_json::Value::Array(array) => array.len(),
            _ => unreachable!(),
        };

        assert_eq!(blogs_len, 0);

        Ok(())
    }

    // UPSERTS

    // "An upsert with a multi-field unique" should "work"
    #[connector_test(schema(schema_multi_uniq))]
    async fn upsert_multi_field_uniq(runner: Runner) -> TestResult<()> {
        let upsert_query = r#"mutation {
          upsertOneUser(where: {
            first_name_last_name: {
              first_name: "The"
              last_name: "Dude"
            }}
            create: {
              id: 1,
              first_name: "The"
              last_name: "Dude"
            }
            update: {
              last_name: { set: "Knight of Ni" }
            }) {
            id
            last_name
          }
        }"#;

        insta::assert_snapshot!(
          run_query!(&runner, upsert_query),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"Dude"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, upsert_query),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"Knight of Ni"}}}"###
        );

        Ok(())
    }

    // "A nested upsert with a multi-field unique" should "work"
    #[connector_test(schema(schema_one2m))]
    async fn nested_upsert_multi_field_uniq(runner: Runner) -> TestResult<()> {
        create_user(&runner, r#"{ id: 1, name: "The Average Leddit User" }"#).await?;

        let upsert_query = r#"mutation {
          updateOneUser(where: {
            id: 1
          }
          data: {
            blogs: {
              upsert: {
                where: {
                  title_category: {
                    title: "How to farm karma with puppy pictures"
                    category: "Pop Culture"
                  }
                }
                create: {
                  id: 1,
                  title: "How to farm karma with puppy pictures"
                  category: "Pop Culture"
                },
                update: {
                  category: { set: "Drama" }
                }
              }
          }}) {
            blogs {
              id
              category
            }
          }
        }"#;

        insta::assert_snapshot!(
          run_query!(&runner, upsert_query),
          @r###"{"data":{"updateOneUser":{"blogs":[{"id":1,"category":"Pop Culture"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, upsert_query),
          @r###"{"data":{"updateOneUser":{"blogs":[{"id":1,"category":"Drama"}]}}}"###
        );

        Ok(())
    }

    // SETS

    // "A nested set with a multi-field unique" should "work"
    #[connector_test(schema(schema_one2m))]
    async fn nested_set_multi_field_uniq(runner: Runner) -> TestResult<()> {
        create_user(&runner, r#"{ id: 1, name: "Ellen Ripley" }"#).await?;
        create_blog(
            &runner,
            r#"{ id: 1, title: "Aliens bad mmmkay" category: "Education" }"#,
        )
        .await?;
        create_blog(
            &runner,
            r#"{ id: 2, title: "Cooking with Aliens" category: "Cooking" }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneUser(
              where: {
                id: 1
              }
              data: {
                blogs:  {
                  set: [{
                    title_category: {
                      title: "Cooking with Aliens"
                      category: "Cooking"
                    }
                  }]
                }
            }) {
              id
              blogs {
                id
              }
            }
          }"#),
          @r###"{"data":{"updateOneUser":{"id":1,"blogs":[{"id":2}]}}}"###
        );

        Ok(())
    }

    async fn create_user(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneUser(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }

    async fn create_blog(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneBlog(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
