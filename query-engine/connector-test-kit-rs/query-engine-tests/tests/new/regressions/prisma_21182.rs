use query_engine_tests::*;

/// Regression test for https://github.com/prisma/prisma/issues/21182
#[test_suite(schema(schema))]
mod regression {
    use indoc::indoc;

    fn schema() -> String {
        indoc! {r#"
            model User {
              #id(id, Int, @id)
              email  String  @unique
              name   String?
              roleId Int
              role   Role    @relation(fields: [roleId], references: [id])
            }

            model Role {
              #id(id, Int, @id)
              name  String
              users User[]

              tagId Int?
              tag   Tag? @relation(fields: [tagId], references: [id])
            }

            model Tag {
              #id(id, Int, @id)
              name  String
              roles Role[]
            }
        "#}
        .to_owned()
    }

    #[connector_test]
    async fn query_with_normalized_dependencies(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"
                mutation {
                  createOneUser (
                    data: {
                      id: 1,
                      email: "user@prisma.io",
                      role: {
                        create: {
                          id: 1,
                          name: "ADMIN",
                          tag: {
                            create: {
                              id: 1,
                              name: "cs"
                            }
                          }
                        }
                      }
                    }
                  ) {
                      id,
                      email,
                      roleId
                  }
                }
            "#}
        );

        run_query!(
            runner,
            indoc! {r#"
                mutation {
                  updateOneUser(
                    where: {
                      email: "user@prisma.io",
                    },
                    data: {
                      role: {
                        update: {
                          data: {
                            tag: {
                              disconnect: true
                            }
                          }
                        }
                      }
                    }
                  ) {
                    id,
                    email,
                    roleId
                  }
                }
            "#}
        );

        Ok(())
    }
}
