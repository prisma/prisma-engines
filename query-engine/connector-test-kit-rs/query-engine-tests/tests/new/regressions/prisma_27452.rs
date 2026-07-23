use query_engine_tests::*;

#[test_suite(schema(schema), exclude(MongoDb, SqlServer))]
mod prisma_27452 {
    fn schema() -> String {
        indoc! {
            r#"
            model User {
              id    String  @id @default(cuid())

              posts        Post[]
              comments     Comment[]
              commentLikes CommentLike[]
            }

            model Post {
              id    String @id @default(cuid())

              user         User          @relation(fields: [ownerId], references: [id], onDelete: Cascade)
              ownerId      String
              comments     Comment[]
              commentLikes CommentLike[]

              @@unique([id, ownerId])
            }

            model Comment {
              id      String @id @default(cuid())

              post         Post          @relation(fields: [postId], references: [id], onDelete: Cascade)
              postId       String
              user         User          @relation(fields: [ownerId], references: [id], onDelete: Cascade)
              ownerId      String
              commentLikes CommentLike[]
            }

            model CommentLike {
              id Int @id

              user     User      @relation(fields: [ownerId], references: [id], onDelete: Cascade)
              ownerId  String
              post     Post      @relation(fields: [postId], references: [id], onDelete: Cascade)
              postId   String
              comments Comment[]

              @@unique([postId, ownerId])
            }
            "#
        }
        .to_string()
    }

    #[connector_test]
    async fn comment_like_upsert_with_nested_comments(runner: Runner) -> TestResult<()> {
        let user_id = "1";
        let post_id = "1";
        let comment1_id = "1";
        let comment2_id = "2";
        let comment_like_id = "1";

        runner
            .query(&format!(
                r#"
                mutation {{
                  createOneUser(data: {{
                    id: "{user_id}"
                    posts: {{
                      createMany: {{
                        data: [{{ id: "{post_id}" }}]
                      }}
                    }}
                    comments: {{
                      createMany: {{
                        data: [
                          {{ id: "{comment1_id}", postId: "{post_id}" }},
                          {{ id: "{comment2_id}", postId: "{post_id}" }}
                        ]
                      }}
                    }}
                  }}) {{
                    id
                  }}
                }}
                "#,
            ))
            .await?;

        let result = runner
            .query(&format!(
                r#"
                mutation {{
                  upsertOneCommentLike(
                    where: {{
                      postId_ownerId: {{ postId: "{post_id}", ownerId: "{user_id}" }}
                    }}
                    create: {{
                      id: {comment_like_id}
                      postId: "{post_id}"
                      ownerId: "{user_id}"
                      comments: {{
                        connect: [{{ id: "{comment1_id}" }}]
                      }}
                    }}
                    update: {{
                      comments: {{
                        set: [{{ id: "{comment2_id}" }}]
                      }}
                    }}
                  ) {{
                    comments {{
                      id
                    }}
                  }}
                }}
                "#,
            ))
            .await?;

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"upsertOneCommentLike":{"comments":[{"id":"1"}]}}}"###
        );

        Ok(())
    }
}
