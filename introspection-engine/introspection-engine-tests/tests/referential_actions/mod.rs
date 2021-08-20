use barrel::types;
use enumflags2::BitFlags;
use expect_test::expect;
use introspection_engine_tests::test_api::*;
use introspection_engine_tests::TestResult;
use test_macros::test_connector;

#[test_connector(tags(Mssql))]
async fn legacy_referential_actions_mssql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES legacy_referential_actions_mssql.a(id) ON DELETE NO ACTION ON UPDATE NO ACTION",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn legacy_referential_actions_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE NO ACTION ON UPDATE NO ACTION",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id])

          @@index([a_id], name: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql))]
async fn legacy_referential_actions(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE NO ACTION ON UPDATE NO ACTION",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(preview_features("referentialActions"), exclude(Mysql, Mssql))]
async fn referential_actions(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE CASCADE ON UPDATE NO ACTION",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(preview_features("referentialActions"), tags(Mysql))]
async fn referential_actions_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE CASCADE ON UPDATE NO ACTION",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction)

          @@index([a_id], name: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(preview_features("referentialActions"), tags(Mssql))]
async fn referential_actions_mssql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES referential_actions_mssql.a(id) ON DELETE CASCADE ON UPDATE NO ACTION",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres, Sqlite), preview_features("referentialActions"))]
async fn default_referential_actions_with_restrict(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));
                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("referentialActions"))]
async fn default_referential_actions_with_restrict_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));
                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id])

          @@index([a_id], name: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("referentialActions"))]
async fn default_referential_actions_without_restrict_mssql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));
                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES default_referential_actions_without_restrict_mssql.a(id) ON DELETE NO ACTION ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("referentialActions"))]
async fn default_optional_actions_mssql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(true));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES default_optional_actions_mssql.a(id) ON DELETE SET NULL ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int  @id @default(autoincrement())
          a_id Int?
          a    a?   @relation(fields: [a_id], references: [id])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("referentialActions"))]
async fn default_optional_actions_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(true));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE SET NULL ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int  @id @default(autoincrement())
          a_id Int?
          a    a?   @relation(fields: [a_id], references: [id])

          @@index([a_id], name: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
