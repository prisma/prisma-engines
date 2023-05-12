mod mssql;
mod mysql;
mod sqlite;

use barrel::types;
use enumflags2::BitFlags;
use expect_test::expect;
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Mssql, Postgres), exclude(CockroachDb))]
async fn introspecting_non_default_pkey_names_works(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Single", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("SomethingCustom", types::primary_constraint(["id"]));
            });

            migration.create_table("Compound", move |t| {
                t.add_column("a", types::integer().increments(false).nullable(false));
                t.add_column("b", types::integer().increments(false).nullable(false));
                t.add_constraint("SomethingCustomCompound", types::primary_constraint(["a", "b"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Compound {
          a Int
          b Int

          @@id([a, b], map: "SomethingCustomCompound")
        }

        model Single {
          id Int @id(map: "SomethingCustom") @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql, Postgres), exclude(CockroachDb))]
async fn introspecting_default_pkey_names_works(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Single", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Single_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Compound", move |t| {
                t.add_column("a", types::integer().increments(false).nullable(false));
                t.add_column("b", types::integer().increments(false).nullable(false));
                t.add_constraint("Compound_pkey", types::primary_constraint(["a", "b"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Compound {
          a Int
          b Int

          @@id([a, b])
        }

        model Single {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql, Postgres), exclude(CockroachDb))]
async fn introspecting_non_default_unique_constraint_names_works(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Single", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("SomethingCustom", types::unique_constraint(["id"]));
            });

            migration.create_table("Compound", move |t| {
                t.add_column("a", types::integer().increments(false).nullable(false));
                t.add_column("b", types::integer().increments(false).nullable(false));
                t.add_constraint("SomethingCustomCompound", types::unique_constraint(["a", "b"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Compound {
          a Int
          b Int

          @@unique([a, b], map: "SomethingCustomCompound")
        }

        model Single {
          id Int @unique(map: "SomethingCustom") @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql, Postgres), exclude(CockroachDb))]
async fn introspecting_default_unique_names_works(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Single", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Single_id_key", types::unique_constraint(["id"]));
            });

            migration.create_table("Compound", move |t| {
                t.add_column("a", types::integer().increments(false).nullable(false));
                t.add_column("b", types::integer().increments(false).nullable(false));
                t.add_constraint("Compound_a_b_key", types::unique_constraint(["a", "b"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Compound {
          a Int
          b Int

          @@unique([a, b])
        }

        model Single {
          id Int @unique @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql, Postgres), exclude(CockroachDb))]
async fn introspecting_non_default_index_names_works(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Single", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_index("SomethingCustom", types::index(["id"]));
            });

            migration.create_table("Compound", move |t| {
                t.add_column("a", types::integer().increments(false).nullable(false));
                t.add_column("b", types::integer().increments(false).nullable(false));
                t.add_index("SomethingCustomCompound", types::index(["a", "b"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Compound {
          a Int
          b Int

          @@index([a, b], map: "SomethingCustomCompound")
          @@ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Single {
          id Int @default(autoincrement())

          @@index([id], map: "SomethingCustom")
          @@ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql, Postgres), exclude(CockroachDb))]
async fn introspecting_default_index_names_works(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Single", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_index("Single_id_idx", types::index(["id"]));
            });

            migration.create_table("Compound", move |t| {
                t.add_column("a", types::integer().increments(false).nullable(false));
                t.add_column("b", types::integer().increments(false).nullable(false));
                t.add_index("Compound_a_b_idx", types::index(["a", "b"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Compound {
          a Int
          b Int

          @@index([a, b])
          @@ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Single {
          id Int @default(autoincrement())

          @@index([id])
          @@ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mssql, Mysql), exclude(CockroachDb))]
async fn introspecting_default_fk_names_works(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_idx", types::index(["user_id"]));
                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int  @id @default(autoincrement())
          user_id Int
          User    User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@index([user_id])
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite, Mssql, Mysql, CockroachDb))]
async fn introspecting_custom_fk_names_works(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_idx", types::index(["user_id"]));
                t.add_constraint(
                    "CustomFKName",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int  @id @default(autoincrement())
          user_id Int
          User    User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "CustomFKName")

          @@index([user_id])
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn introspecting_custom_default_names_should_output_to_dml(api: &mut TestApi) -> TestResult {
    let create_table = format!(
        "CREATE TABLE [{}].[custom_defaults_test] (id INT CONSTRAINT pk_meow PRIMARY KEY, data NVARCHAR(255) CONSTRAINT meow DEFAULT 'foo')",
        api.schema_name()
    );

    api.database().raw_cmd(&create_table).await?;

    let expected = expect![[r#"
        model custom_defaults_test {
          id   Int     @id(map: "pk_meow")
          data String? @default("foo", map: "meow") @db.NVarChar(255)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn introspecting_default_default_names_should_not_output_to_dml(api: &mut TestApi) -> TestResult {
    let create_table = format!(
        "CREATE TABLE [{}].[custom_defaults_test] (id INT CONSTRAINT pk_meow PRIMARY KEY, data NVARCHAR(255) CONSTRAINT custom_defaults_test_data_df DEFAULT 'foo')",
        api.schema_name()
    );

    api.database().raw_cmd(&create_table).await?;

    let expected = expect![[r#"
        model custom_defaults_test {
          id   Int     @id(map: "pk_meow")
          data String? @default("foo") @db.NVarChar(255)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
