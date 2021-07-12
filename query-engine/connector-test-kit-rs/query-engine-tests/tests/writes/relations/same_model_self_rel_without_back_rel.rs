use query_engine_tests::*;

#[test_suite(schema(schema))]
mod self_rel_no_back_rel {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Post {
              #id(id, String, @id)
              identifier Int?   @unique
              related    Post[] @relation(name: "RelatedPosts")
              parents    Post[] @relation(name: "RelatedPosts")
            }"#
        };

        schema.to_owned()
    }

    // "A Many to Many Self Relation" should "be accessible from only one side"
    // Bring back sql server when cascading rules can be set!
    // TODO(dom): Not working on mongo (both snapshots)
    //-{"data":{"findUniquePost":{"identifier":1,"related":[{"identifier":2}]}}}
    //{"errors":[{"error":"called `Option::unwrap()` on a `None` value","user_facing_error":{"is_panic":true,"message":"called `Option::unwrap()` on a `None` value","backtrace":null}}]}
    #[connector_test(exclude(SqlServer, MongoDb))]
    async fn m2m_self_rel(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{id: "1", identifier: 1}"#).await?;
        create_row(runner, r#"{id: "2", identifier: 2}"#).await?;

        run_query!(
            runner,
            r#"mutation {
            updateOnePost (
              where:{ identifier: 1 }
              data: {
                related: {
                  connect: {
                    identifier: 2
                  }
                }
              }
            ) {
              identifier
            }
          }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{findUniquePost(where:{identifier: 1}){identifier, related{identifier}}}"#),
          @r###"{"data":{"findUniquePost":{"identifier":1,"related":[{"identifier":2}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{findUniquePost(where:{identifier: 2}){identifier, related{identifier}}}"#),
          @r###"{"data":{"findUniquePost":{"identifier":2,"related":[]}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model Post {
              #id(id, String, @id)
              identifier Int?   @unique
              relatedId  String?

              related    Post?  @relation(name: "RelatedPosts", fields:[relatedId], references: [id])
              parents    Post[] @relation(name: "RelatedPosts")
            }"#
        };

        schema.to_owned()
    }

    // "A One to One Self Relation" should "be accessible from only one side"
    // TODO: Why is it failing to migrate the dm in rust but not in Scala for MSSQL??
    #[connector_test(schema(schema_2), exclude(SqlServer))]
    async fn one2one_self_rel(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{id: "1", identifier: 1}"#).await?;
        create_row(runner, r#"{id: "2", identifier: 2}"#).await?;
        run_query!(
            runner,
            r#"mutation {
          updateOnePost (
            where:{identifier: 1}
            data: {
              related: {
                connect: {
                  identifier: 2
                }
              }
            }
          ) {
            identifier
          }
        }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{findUniquePost(where:{identifier: 1}){identifier, related{identifier}}}"#),
          @r###"{"data":{"findUniquePost":{"identifier":1,"related":{"identifier":2}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{findUniquePost(where:{identifier: 2}){identifier, related{identifier}}}"#),
          @r###"{"data":{"findUniquePost":{"identifier":2,"related":null}}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOnePost(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
