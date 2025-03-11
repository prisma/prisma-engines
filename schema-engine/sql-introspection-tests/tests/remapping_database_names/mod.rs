mod mssql;
mod mysql;
mod postgresql;
mod sqlite;

use barrel::types;
use expect_test::expect;
use indoc::formatdoc;
use indoc::indoc;
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(exclude(CockroachDb))]
async fn remapping_fields_with_invalid_characters(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("_a", types::text());
                t.add_column("*b", types::text());
                t.add_column("?c", types::text());
                t.add_column("(d", types::text());
                t.add_column(")e", types::text());
                t.add_column("/f", types::text());
                t.add_column("g a", types::text());
                t.add_column("h-a", types::text());
                t.add_column("h1", types::text());

                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let native_string = if api.sql_family().is_mssql() || api.sql_family().is_mysql() {
        "@db.Text"
    } else {
        ""
    };

    let dm = formatdoc! {r#"
        model User {{
            id     Int @id @default(autoincrement())
            a      String @map("_a") {native_string}
            b      String @map("*b") {native_string}
            c      String @map("?c") {native_string}
            d      String @map("(d") {native_string}
            e      String @map(")e") {native_string}
            f      String @map("/f") {native_string}
            g_a    String @map("g a") {native_string}
            h_a    String @map("h-a") {native_string}
            h1     String {native_string}
        }}
    "#, native_string = native_string};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(&dm, &result);

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn remapping_tables_with_invalid_characters(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("?User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("?User_pkey", types::primary_constraint(vec!["id"]))
            });

            migration.create_table("User with Space", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User with Space_pkey", types::primary_constraint(vec!["id"]))
            });
        })
        .await?;

    let dm = indoc! {r#"
        model User {
            id      Int @id @default(autoincrement())

            @@map("?User")
        }

        model User_with_Space {
            id      Int @id @default(autoincrement())

            @@map("User with Space")
        }
    "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(exclude(Mssql, Sqlite, Vitess, CockroachDb))]
async fn remapping_models_in_relations(api: &mut TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());

                if sql_family.is_mysql() {
                    t.inject_custom(
                        "CONSTRAINT asdf FOREIGN KEY (user_id) REFERENCES `User with Space`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                    );
                } else {
                    t.inject_custom(
                        r#"CONSTRAINT asdf FOREIGN KEY (user_id) REFERENCES "User with Space"(id) ON DELETE RESTRICT ON UPDATE CASCADE"#,
                    );
                }

                t.add_constraint(
                    "post_user_unique",
                    types::unique_constraint(vec!["user_id"]).unique(true),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id              Int             @id @default(autoincrement())
          user_id         Int             @unique(map: "post_user_unique")
          User_with_Space User_with_Space @relation(fields: [user_id], references: [id], map: "asdf")
        }

        model User_with_Space {
          id   Int   @id @default(autoincrement())
          Post Post?

          @@map("User with Space")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mssql, Sqlite, Vitess, CockroachDb))]
async fn remapping_models_in_relations_should_not_map_virtual_fields(api: &mut TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]))
            });

            migration.create_table("Post With Space", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer());

                if sql_family.is_mysql() {
                    t.inject_custom(
                        "CONSTRAINT asdf FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                    );
                } else {
                    t.inject_custom(
                        r#"CONSTRAINT asdf FOREIGN KEY (user_id) REFERENCES "User"(id) ON DELETE RESTRICT ON UPDATE CASCADE"#,
                    );
                }


                t.add_constraint("post_user_unique", types::unique_constraint(vec!["user_id"]));
                t.add_constraint("Post With Space_pkey", types::primary_constraint(vec!["id"]))
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post_With_Space {
          id      Int  @id @default(autoincrement())
          user_id Int  @unique(map: "post_user_unique")
          User    User @relation(fields: [user_id], references: [id], map: "asdf")

          @@map("Post With Space")
        }

        model User {
          id              Int              @id @default(autoincrement())
          Post_With_Space Post_With_Space?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite, Mssql, Vitess, CockroachDb))]
async fn remapping_fields_in_compound_relations(api: &mut TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age-that-is-invalid", types::integer());

                t.add_constraint(
                    "user_unique",
                    types::unique_constraint(vec!["id", "age-that-is-invalid"]),
                );
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                if sql_family.is_mysql() {
                    t.inject_custom(
                        "CONSTRAINT asdf FOREIGN KEY (user_id, user_age) REFERENCES User(id, `age-that-is-invalid`) ON DELETE RESTRICT ON UPDATE CASCADE",
                    );
                } else {
                    t.inject_custom(
                        r#"CONSTRAINT asdf FOREIGN KEY (user_id, user_age) REFERENCES "User"(id, "age-that-is-invalid") ON DELETE RESTRICT ON UPDATE CASCADE"#,
                    );
                }

                t.add_constraint(
                    "post_user_unique",
                    types::unique_constraint(vec!["user_id", "user_age"]),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id       Int  @id @default(autoincrement())
          user_id  Int
          user_age Int
          User     User @relation(fields: [user_id, user_age], references: [id, age_that_is_invalid], map: "asdf")

          @@unique([user_id, user_age], map: "post_user_unique")
        }

        model User {
          id                  Int   @id @default(autoincrement())
          age_that_is_invalid Int   @map("age-that-is-invalid")
          Post                Post?

          @@unique([id, age_that_is_invalid], map: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(capabilities(Enums), exclude(CockroachDb, Sqlite))]
async fn remapping_enum_values(api: &mut TestApi) -> TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.database()
            .execute_raw("CREATE TYPE color AS ENUM ('b lack', 'w hite')", &[])
            .await?;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let typ = if sql_family.is_mysql() {
                    "ENUM ('b lack', 'w hite')"
                } else {
                    "color"
                };

                t.add_column("color", types::custom(typ).nullable(true));
            });
        })
        .await?;

    let enum_name = if sql_family.is_mysql() { "Book_color" } else { "color" };

    let dm = format!(
        r#"
        model Book {{
            id      Int  @id @default(autoincrement())
            color   {enum_name}?
        }}

        enum {enum_name} {{
            b_lack   @map("b lack")
            w_hite   @map("w hite")
        }}
    "#
    );

    let result = api.introspect().await?;
    api.assert_eq_datamodels(&dm, &result);

    Ok(())
}

#[test_connector(capabilities(Enums), exclude(CockroachDb, Sqlite))]
async fn remapping_enum_default_values(api: &mut TestApi) -> TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.database()
            .execute_raw("CREATE TYPE color AS ENUM ('b lack', 'white')", &[])
            .await?;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let typ = if sql_family.is_mysql() {
                    "ENUM ('b lack', 'white')"
                } else {
                    "color"
                };

                t.add_column("color", types::custom(typ).nullable(false).default("b lack"));
            });
        })
        .await?;

    let enum_name = if sql_family.is_mysql() { "Book_color" } else { "color" };

    let dm = format!(
        r#"
        model Book {{
            id      Int @id @default(autoincrement())
            color   {enum_name} @default(b_lack)
        }}

        enum {enum_name} {{
            b_lack @map("b lack")
            white
        }}
    "#
    );

    let result = api.introspect().await?;
    api.assert_eq_datamodels(&dm, &result);

    Ok(())
}

#[test_connector]
async fn remapping_compound_primary_keys(api: &mut TestApi) -> TestResult {
    api.normalise_int_type().await?;

    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("first_name", types::integer());
                t.add_column("last@name", types::integer());
                t.add_constraint("User_pkey", types::primary_constraint(vec!["first_name", "last@name"]));
            });
        })
        .await?;

    let dm = indoc! {r#"
        model User {
            first_name  Int
            last_name   Int @map("last@name")

            @@id([first_name, last_name])
        }
    "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn not_automatically_remapping_invalid_compound_unique_key_names(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
                t.add_column("first", types::integer());
                t.add_column("last", types::integer());
                t.add_index(
                    "User.something@invalid-and/weird",
                    types::index(["first", "last"]).unique(true),
                );
            });
        })
        .await?;

    let dm = indoc! {r#"
         model User {
             id     Int @id @default(autoincrement())
             first  Int
             last   Int

             @@unique([first, last], map: "User.something@invalid-and/weird")
         }
     "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector]
async fn not_automatically_remapping_invalid_compound_primary_key_names(api: &mut TestApi) -> TestResult {
    api.normalise_int_type().await?;

    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("first", types::integer());
                t.add_column("last", types::integer());
                t.add_constraint(
                    "User.something@invalid-and/weird",
                    types::primary_constraint(["first", "last"]).unique(true),
                );
            });
        })
        .await?;

    let pk_name = if api.sql_family().is_sqlite() || api.sql_family().is_mysql() {
        ""
    } else {
        ", map: \"User.something@invalid-and/weird\""
    };

    let dm = format! {r#"
         model User {{
             first  Int
             last   Int

             @@id([first, last]{pk_name})
         }}
     "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(&dm, &result);

    Ok(())
}
