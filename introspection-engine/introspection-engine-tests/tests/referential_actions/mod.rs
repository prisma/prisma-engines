mod sqlite;

use barrel::types;
use enumflags2::BitFlags;
use expect_test::expect;
use introspection_engine_tests::test_api::*;
use introspection_engine_tests::TestResult;
use test_macros::test_connector;

#[test_connector(exclude(Mysql, Mssql, Sqlite))]
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
          a    a   @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction, map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
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
          a    a   @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction, map: "asdf")

          @@index([a_id], map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn referential_actions_mssql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("a_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("a_id", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES referential_actions_mssql.a(id) ON DELETE CASCADE ON UPDATE NO ACTION",
                );
                t.add_constraint("b_pkey", types::primary_constraint(vec!["id"]));
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
          a    a   @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction, map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn default_referential_actions_with_restrict_postgres(api: &TestApi) -> TestResult {
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
          a    a   @relation(fields: [a_id], references: [id], map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn default_referential_actions_with_restrict_sqlite(api: &TestApi) -> TestResult {
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

#[test_connector(tags(Mysql), exclude(Vitess))]
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
          a    a   @relation(fields: [a_id], references: [id], map: "asdf")

          @@index([a_id], map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn default_referential_actions_without_restrict_mssql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("a_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("a_id", types::integer().nullable(false));
                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES default_referential_actions_without_restrict_mssql.a(id) ON DELETE NO ACTION ON UPDATE CASCADE",
                );
                t.add_constraint("b_pkey", types::primary_constraint(vec!["id"]));
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
          a    a   @relation(fields: [a_id], references: [id], map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn default_optional_actions_mssql(api: &TestApi) -> TestResult {
    let setup = r#"
       CREATE TABLE [default_optional_actions_mssql].[a] (
            [id] INTEGER IDENTITY,
            CONSTRAINT a_pkey PRIMARY KEY CLUSTERED ([id])
        );

        CREATE TABLE [default_optional_actions_mssql].[b] (
            [id] INTEGER IDENTITY,
            [a_id] INTEGER,

            CONSTRAINT b_pkey PRIMARY KEY CLUSTERED ([id]),
            CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES default_optional_actions_mssql.a(id) ON DELETE SET NULL ON UPDATE CASCADE
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model a {
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int  @id @default(autoincrement())
          a_id Int?
          a    a?   @relation(fields: [a_id], references: [id], map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
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
          a    a?   @relation(fields: [a_id], references: [id], map: "asdf")

          @@index([a_id], map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
