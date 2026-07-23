mod cockroachdb;
mod mysql;
mod sqlite;

use barrel::types;
use enumflags2::BitFlags;
use expect_test::expect;
use sql_introspection_tests::TestResult;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(exclude(Mysql, Mssql, Sqlite, CockroachDb))]
async fn referential_actions(api: &mut TestApi) -> TestResult {
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
async fn referential_actions_mysql(api: &mut TestApi) -> TestResult {
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

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn default_referential_actions_with_restrict_postgres(api: &mut TestApi) -> TestResult {
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
async fn default_referential_actions_with_restrict_sqlite(api: &mut TestApi) -> TestResult {
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
async fn default_referential_actions_with_restrict_mysql(api: &mut TestApi) -> TestResult {
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

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn default_optional_actions_mysql(api: &mut TestApi) -> TestResult {
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
