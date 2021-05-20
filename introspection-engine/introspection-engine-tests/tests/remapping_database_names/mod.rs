use barrel::types;
use datamodel::ReferentialAction;
use indoc::formatdoc;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use quaint::prelude::{Queryable, SqlFamily};
use test_macros::test_connector;

#[test_connector(tags(Postgres))]
async fn should_not_remap_if_renaming_would_lead_to_duplicate_names(api: &TestApi) -> TestResult {
    api.database()
        .raw_cmd("CREATE TABLE nodes(id serial primary key)")
        .await?;

    api.database()
        .raw_cmd(
            "CREATE TABLE _nodes(
                node_a int not null,
                node_b int not null,
                constraint _nodes_node_a_fkey foreign key(node_a) references nodes(id) on delete cascade on update cascade,
                constraint _nodes_node_b_fkey foreign key(node_b) references nodes(id) on delete cascade on update cascade
            )
        ",
        )
        .await?;

    assert!(api.introspect().await.is_ok());

    Ok(())
}

#[test_connector]
async fn remapping_fields_with_invalid_characters(api: &TestApi) -> TestResult {
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

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn remapping_tables_with_invalid_characters(api: &TestApi) -> TestResult {
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn remapping_models_in_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
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

    let action = match api.sql_family() {
        SqlFamily::Mysql if !api.is_mysql8() => ReferentialAction::Restrict,
        _ => ReferentialAction::NoAction,
    };

    let dm = formatdoc!(
        r#"
        model Post {{
            id              Int             @id @default(autoincrement())
            user_id         Int             @unique
            User_with_Space User_with_Space @relation(fields: [user_id], references: [id], onDelete: {action}, onUpdate: {action})
        }}

        model User_with_Space {{
            id   Int    @id @default(autoincrement())
            Post Post?

            @@map("User with Space")
        }}
    "#,
        action = action
    );

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn remapping_models_in_relations_should_not_map_virtual_fields(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post With Space", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_foreign_key(&["user_id"], "User", &["id"]);

                t.add_constraint("post_user_unique", types::unique_constraint(vec!["user_id"]));
            });
        })
        .await?;

    let action = match api.sql_family() {
        SqlFamily::Mysql if !api.is_mysql8() => ReferentialAction::Restrict,
        _ => ReferentialAction::NoAction,
    };

    let dm = formatdoc!(
        r#"
        model Post_With_Space {{
            id      Int  @id @default(autoincrement())
            user_id Int  @unique
            User    User @relation(fields: [user_id], references: [id], onDelete: {action}, onUpdate: {action})

            @@map("Post With Space")
        }}

        model User {{
            id              Int              @id @default(autoincrement())
            Post_With_Space Post_With_Space?
        }}
    "#,
        action = action
    );

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn remapping_models_in_compound_relations(api: &TestApi) -> TestResult {
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

    let action = match api.sql_family() {
        SqlFamily::Mysql if !api.is_mysql8() => ReferentialAction::Restrict,
        _ => ReferentialAction::NoAction,
    };

    let dm = format!(
        r#"
        model Post {{
            id              Int             @id @default(autoincrement())
            user_id         Int
            user_age        Int
            User_with_Space User_with_Space @relation(fields: [user_id, user_age], references: [id, age], onDelete: {action}, onUpdate: {action})

            @@unique([user_id, user_age], name: "{post_constraint}")
        }}

        model User_with_Space {{
            id   Int   @id @default(autoincrement())
            age  Int
            Post Post?

            @@map("User with Space")
            @@unique([id, age], name: "{user_constraint}")
        }}
    "#,
        post_constraint = post_constraint,
        user_constraint = user_constraint,
        action = action
    );

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn remapping_fields_in_compound_relations(api: &TestApi) -> TestResult {
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

    let action = match api.sql_family() {
        SqlFamily::Mysql if !api.is_mysql8() => ReferentialAction::Restrict,
        _ => ReferentialAction::NoAction,
    };

    let dm = format!(
        r#"
        model Post {{
            id       Int  @id @default(autoincrement())
            user_id  Int
            user_age Int
            User     User @relation(fields: [user_id, user_age], references: [id, age_that_is_invalid], onDelete: {action}, onUpdate: {action})

            @@unique([user_id, user_age], name: "{user_post_constraint}")
        }}

        model User {{
            id                  Int   @id @default(autoincrement())
            age_that_is_invalid Int   @map("age-that-is-invalid")
            Post                Post?

            @@unique([id, age_that_is_invalid], name: "{user_constraint}")
        }}
    "#,
        user_post_constraint = user_post_constraint,
        user_constraint = user_constraint,
        action = action
    );

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(capabilities(Enums))]
async fn remapping_enum_names(api: &TestApi) -> TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.database()
            .raw_cmd("CREATE TYPE \"123color\" AS ENUM ('black')")
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

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(capabilities(Enums))]
async fn remapping_enum_values(api: &TestApi) -> TestResult {
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

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(capabilities(Enums))]
async fn remapping_enum_default_values(api: &TestApi) -> TestResult {
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

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn remapping_compound_primary_keys(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("first_name", types::integer());
                t.add_column("last@name", types::integer());
                t.set_primary_key(&["first_name", "last@name"]);
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}
