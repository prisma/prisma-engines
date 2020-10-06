use barrel::types;
use indoc::indoc;
use introspection_engine_tests::{assert_eq_datamodels, test_api::*};
use quaint::prelude::Queryable;
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector]
async fn remapping_fields_with_invalid_characters(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("_a", types::text());
                t.add_column("*b", types::text());
                t.add_column("?c", types::text());
                t.add_column("(d", types::text());
                t.add_column(")e", types::text());
                t.add_column("/f", types::text());
                t.add_column("g a", types::text());
                t.add_column("h-a", types::text());
                t.add_column("h1", types::text());
            });
        })
        .await?;

    let dm = indoc! {r#"
        model User {
            id     Int @id @default(autoincrement())
            a      String @map("_a")
            b      String @map("*b")
            c      String @map("?c")
            d      String @map("(d")
            e      String @map(")e")
            f      String @map("/f")
            g_a    String @map("g a")
            h_a    String @map("h-a")
            h1     String
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn remapping_tables_with_invalid_characters(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("?User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
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

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn remapping_models_in_relations(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_foreign_key(&["user_id"], "User with Space", &["id"]);

                t.add_constraint(
                    "post_user_unique",
                    types::unique_constraint(vec!["user_id"]).unique(true),
                );
            });
        })
        .await?;

    let dm = {
        r#"
        model Post {
            id              Int             @id @default(autoincrement())
            user_id         Int             @unique
            User_with_Space User_with_Space @relation(fields: [user_id], references: [id])
        }

        model User_with_Space {
            id   Int    @id @default(autoincrement())
            name String
            Post Post?

            @@map("User with Space")
        }
    "#
    };

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn remapping_models_in_relations_should_not_map_virtual_fields(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
            });

            migration.create_table("Post With Space", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_foreign_key(&["user_id"], "User", &["id"]);

                t.add_constraint("post_user_unique", types::unique_constraint(vec!["user_id"]));
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Post_With_Space {
            id      Int  @id @default(autoincrement())
            user_id Int  @unique
            User    User @relation(fields: [user_id], references: [id])

            @@map("Post With Space")
        }

        model User {
            id              Int              @id @default(autoincrement())
            name            String
            Post_With_Space Post_With_Space?
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn remapping_models_in_compound_relations(api: &TestApi) -> crate::TestResult {
    let post_constraint = if api.sql_family().is_sqlite() {
        "sqlite_autoindex_Post_1"
    } else {
        "post_user_unique"
    };

    let user_constraint = if api.sql_family().is_sqlite() {
        "sqlite_autoindex_User with Space_1"
    } else {
        "user_unique"
    };

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User with Space", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_constraint(user_constraint, types::unique_constraint(vec!["id", "age"]));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_foreign_key(&["user_id", "user_age"], "User with Space", &["id", "age"]);

                t.add_constraint(
                    post_constraint,
                    types::unique_constraint(vec!["user_id", "user_age"]).unique(true),
                );
            });
        })
        .await?;

    let dm = format!(
        r#"
        model Post {{
            id              Int             @id @default(autoincrement())
            user_id         Int
            user_age        Int
            User_with_Space User_with_Space @relation(fields: [user_id, user_age], references: [id, age])

            @@unique([user_id, user_age], name: "{}")
        }}

        model User_with_Space {{
            id   Int   @id @default(autoincrement())
            age  Int
            Post Post?

            @@map("User with Space")
            @@unique([id, age], name: "{}")
        }}
    "#,
        post_constraint, user_constraint
    );

    assert_eq_datamodels!(&dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn remapping_fields_in_compound_relations(api: &TestApi) -> crate::TestResult {
    let user_post_constraint = if api.sql_family().is_sqlite() {
        "sqlite_autoindex_Post_1"
    } else {
        "post_user_unique"
    };

    let user_constraint = if api.sql_family().is_sqlite() {
        "sqlite_autoindex_User_1"
    } else {
        "user_unique"
    };

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age-that-is-invalid", types::integer());

                t.add_constraint(
                    user_constraint,
                    types::unique_constraint(vec!["id", "age-that-is-invalid"]),
                );
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_foreign_key(&["user_id", "user_age"], "User", &["id", "age-that-is-invalid"]);

                t.add_constraint(
                    user_post_constraint,
                    types::unique_constraint(vec!["user_id", "user_age"]),
                );
            });
        })
        .await?;

    let dm = format!(
        r#"
        model Post {{
            id       Int  @id @default(autoincrement())
            user_id  Int
            user_age Int
            User     User @relation(fields: [user_id, user_age], references: [id, age_that_is_invalid])

            @@unique([user_id, user_age], name: "{}")
        }}

        model User {{
            id                  Int   @id @default(autoincrement())
            age_that_is_invalid Int   @map("age-that-is-invalid")
            Post                Post?

            @@unique([id, age_that_is_invalid], name: "{}")
        }}
    "#,
        user_post_constraint, user_constraint
    );

    assert_eq_datamodels!(&dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn remapping_enum_names(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.database()
            .execute_raw("CREATE TYPE \"123color\" AS ENUM ('black')", &[])
            .await?;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("123Book", move |t| {
                t.add_column("id", types::primary());

                let typ = if sql_family.is_mysql() {
                    "ENUM ('black')"
                } else {
                    "\"123color\""
                };

                t.add_column("1color", types::custom(typ).nullable(true));
            });
        })
        .await?;

    let enum_name = if sql_family.is_mysql() { "Book_color" } else { "color" };

    let renamed_enum = if sql_family.is_mysql() {
        "123Book_1color"
    } else {
        "123color"
    };

    let dm = format!(
        r#"
        model Book {{
            id      Int @id @default(autoincrement())
            color   {0}? @map("1color")

            @@map("123Book")
        }}

        enum {0} {{
            black
            @@map("{1}")
        }}
    "#,
        enum_name, renamed_enum
    );

    assert_eq_datamodels!(&dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn remapping_enum_values(api: &TestApi) -> crate::TestResult {
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
            color   {0}?
        }}

        enum {0} {{
            b_lack   @map("b lack")
            w_hite   @map("w hite")
        }}
    "#,
        enum_name
    );

    assert_eq_datamodels!(&dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn remapping_enum_default_values(api: &TestApi) -> crate::TestResult {
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
            color   {0} @default(b_lack)
        }}

        enum {0} {{
            b_lack @map("b lack")
            white
        }}
    "#,
        enum_name
    );

    assert_eq_datamodels!(&dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn remapping_compound_primary_keys(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("first_name", types::varchar(255));
                t.add_column("last@name", types::varchar(255));
                t.set_primary_key(&["first_name", "last@name"]);
            });
        })
        .await?;

    let dm = indoc! {r#"
        model User {
            first_name  String
            last_name   String @map("last@name")

            @@id([first_name, last_name])
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}
