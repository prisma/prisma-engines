use barrel::types;
use indoc::indoc;
use introspection_engine_tests::{assert_eq_datamodels, test_api::*};
use quaint::prelude::SqlFamily;
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector]
async fn one_to_one_req_relation(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("Post", move |t| {
                    t.add_column("id", types::primary());
                    t.add_column("user_id", types::integer().nullable(false).unique(true));
                    t.add_foreign_key(&["user_id"], "User", &["id"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Post {
            id       Int @id @default(autoincrement())
            user_id  Int  @unique
            User     User @relation(fields: [user_id], references: [id])
        }

        model User {
            id      Int @id @default(autoincrement())
            Post Post?
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn one_to_one_relation_on_a_singular_primary_key(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("Post", |t| {
                    t.add_column("id", types::integer().nullable(false).unique(true));
                    t.add_foreign_key(&["id"], "User", &["id"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Post {
            id   Int  @unique
            User User @relation(fields: [id], references: [id])
        }

        model User {
            id   Int   @id @default(autoincrement())
            Post Post?
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn two_one_to_one_relations_between_the_same_models(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", move |t| {
                    t.add_column("id", types::primary());
                    t.add_column("post_id", types::integer().unique(true).nullable(false));

                    // Other databases can't create a foreign key before the table
                    // exists, SQLite can, but cannot alter table with a foreign
                    // key.
                    if sql_family.is_sqlite() {
                        t.add_foreign_key(&["post_id"], "Post", &["id"]);
                    }
                });

                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("user_id", types::integer().unique(true).nullable(false));
                    t.add_foreign_key(&["user_id"], "User", &["id"]);
                });

                // Other databases can't create a foreign key before the table
                // exists, SQLite can, but cannot alter table with a foreign
                // key.
                if !sql_family.is_sqlite() {
                    migration.change_table("User", |t| {
                        t.add_foreign_key(&["post_id"], "Post", &["id"]);
                    })
                }
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Post {
            id                      Int   @id @default(autoincrement())
            user_id                 Int   @unique
            User_Post_user_idToUser User  @relation("Post_user_idToUser", fields: [user_id], references: [id])
            User_PostToUser_post_id User? @relation("PostToUser_post_id")
        }

        model User {
            id                      Int   @id @default(autoincrement())
            post_id                 Int   @unique
            Post_PostToUser_post_id Post  @relation("PostToUser_post_id", fields: [post_id], references: [id])
            Post_Post_user_idToUser Post? @relation("Post_user_idToUser")
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_one_to_one_relation(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("user_id", types::integer().unique(true).nullable(true));
                    t.add_foreign_key(&["user_id"], "User", &["id"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Post {
            id      Int  @id @default(autoincrement())
            user_id Int?  @unique
            User    User? @relation(fields: [user_id], references: [id])
        }

        model User {
            id   Int   @id @default(autoincrement())
            Post Post?
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_one_to_one_relation_referencing_non_id(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("email", types::varchar(10).unique(true).nullable(true));
                });

                migration.create_table("Post", move |t| {
                    t.add_column("id", types::primary());
                    t.add_column("user_email", types::varchar(10).unique(true).nullable(true));
                    t.add_foreign_key(&["user_email"], "User", &["email"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Post {
            id         Int     @id @default(autoincrement())
            user_email String? @unique
            User       User?   @relation(fields: [user_email], references: [email])
        }

        model User {
            id    Int     @id @default(autoincrement())
            email String? @unique
            Post  Post?
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_one_to_many_relation(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("user_id", types::integer().unique(false).nullable(true));
                    t.add_foreign_key(&["user_id"], "User", &["id"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = match api.sql_family() {
        SqlFamily::Mysql => {
            indoc! {r##"
                model Post {
                    id      Int   @id @default(autoincrement())
                    user_id Int?
                    User    User? @relation(fields: [user_id], references: [id])
                    @@index([user_id], name: "user_id")
                }

                model User {
                    id   Int    @id @default(autoincrement())
                    Post Post[]
                }
            "##}
        }
        _ => {
            indoc! {r##"
                model Post {
                    id      Int   @id @default(autoincrement())
                    user_id Int?
                    User    User? @relation(fields: [user_id], references: [id])
                }

                model User {
                    id   Int    @id @default(autoincrement())
                    Post Post[]
                }
            "##}
        }
    };

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_one_req_to_many_relation(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("user_id", types::integer().unique(false).nullable(false));
                    t.add_foreign_key(&["user_id"], "User", &["id"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = match api.sql_family() {
        SqlFamily::Mysql => {
            indoc! {r##"
                model Post {
                    id      Int   @id @default(autoincrement())
                    user_id Int
                    User    User @relation(fields: [user_id], references: [id])
                    @@index([user_id], name: "user_id")
                }

                model User {
                    id   Int    @id @default(autoincrement())
                    Post Post[]
                }
            "##}
        }
        _ => {
            indoc! {r##"
                model Post {
                    id      Int   @id @default(autoincrement())
                    user_id Int
                    User    User @relation(fields: [user_id], references: [id])
                }

                model User {
                    id   Int    @id @default(autoincrement())
                    Post Post[]
                }
            "##}
        }
    };

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_prisma_many_to_many_relation(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("_PostToUser", |t| {
                    t.add_column("A", types::integer().nullable(false).unique(false));
                    t.add_column("B", types::integer().nullable(false).unique(false));

                    t.add_foreign_key(&["A"], "Post", &["id"]);
                    t.add_foreign_key(&["B"], "User", &["id"]);

                    t.add_index("test", types::index(vec!["A", "B"]).unique(true));
                    t.add_index("test2", types::index(vec!["B"]).unique(false));
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Post {
            id   Int    @id @default(autoincrement())
            User User[]
        }

        model User {
            id   Int    @id @default(autoincrement())
            Post Post[]
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_many_to_many_relation_with_an_id(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("PostsToUsers", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("user_id", types::integer().nullable(false));
                    t.add_column("post_id", types::integer().nullable(false));

                    t.add_foreign_key(&["user_id"], "User", &["id"]);
                    t.add_foreign_key(&["post_id"], "Post", &["id"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = match api.sql_family() {
        SqlFamily::Mysql => {
            indoc! {r##"
                model Post {
                    id           Int            @id @default(autoincrement())
                    PostsToUsers PostsToUsers[]
                }

                model PostsToUsers {
                    id      Int  @id @default(autoincrement())
                    user_id Int
                    post_id Int
                    Post    Post @relation(fields: [post_id], references: [id])
                    User    User @relation(fields: [user_id], references: [id])
                    @@index([post_id], name: "post_id")
                    @@index([user_id], name: "user_id")
                }

                model User {
                    id           Int            @id @default(autoincrement())
                    PostsToUsers PostsToUsers[]
                }         
            "##}
        }
        _ => {
            indoc! {r##"
                model Post {
                    id           Int            @id @default(autoincrement())
                    PostsToUsers PostsToUsers[]
                }

                model PostsToUsers {
                    id      Int  @id @default(autoincrement())
                    user_id Int
                    post_id Int
                    Post    Post @relation(fields: [post_id], references: [id])
                    User    User @relation(fields: [user_id], references: [id])
                }

                model User {
                    id           Int            @id @default(autoincrement())
                    PostsToUsers PostsToUsers[]
                }         
            "##}
        }
    };

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_self_relation(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", move |t| {
                    t.add_column("id", types::primary());
                    t.add_column("recruited_by", types::integer().nullable(true));
                    t.add_column("direct_report", types::integer().nullable(true));

                    t.add_foreign_key(&["recruited_by"], "User", &["id"]);
                    t.add_foreign_key(&["direct_report"], "User", &["id"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = match api.sql_family() {
        SqlFamily::Mysql => {
            indoc! {r##"
                model User {
                    id                                  Int    @id @default(autoincrement())
                    recruited_by                        Int?
                    direct_report                       Int?
                    User_UserToUser_direct_report       User?  @relation("UserToUser_direct_report", fields: [direct_report], references: [id])
                    User_UserToUser_recruited_by        User?  @relation("UserToUser_recruited_by", fields: [recruited_by], references: [id])
                    other_User_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
                    other_User_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")
                    @@index([direct_report], name: "direct_report")
                    @@index([recruited_by], name: "recruited_by")
                }
            "##}
        }
        _ => {
            indoc! {r##"
                model User {
                    id                                  Int    @id @default(autoincrement())
                    recruited_by                        Int?
                    direct_report                       Int?
                    User_UserToUser_direct_report       User?  @relation("UserToUser_direct_report", fields: [direct_report], references: [id])
                    User_UserToUser_recruited_by        User?  @relation("UserToUser_recruited_by", fields: [recruited_by], references: [id])
                    other_User_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
                    other_User_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")
                }
            "##}
        }
    };

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

// SQLite will always make the primary key autoincrement, which makes no sense
// to build.
#[test_each_connector(ignore("sqlite"))]
async fn id_fields_with_foreign_key(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            move |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });
                migration.create_table("Post", move |t| {
                    t.add_column("user_id", types::integer().primary(true));
                    t.add_foreign_key(&["user_id"], "User", &["id"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Post {
            user_id Int    @id
            User    User   @relation(fields: [user_id], references: [id])
        }

        model User {
            id   Int    @id @default(autoincrement())
            Post Post?
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

// SQLite cannot alter tables to add foreign keys, so skipping the tests.
#[test_each_connector(ignore("sqlite"))]
async fn duplicate_fks_should_ignore_one_of_them(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                });

                migration.create_table("Post", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("user_id", types::integer().nullable(true));
                    t.add_foreign_key(&["user_id"], "User", &["id"]);
                });

                migration.change_table("Post", |t| {
                    t.add_foreign_key(&["user_id"], "User", &["id"]);
                })
            },
            api.schema_name(),
        )
        .await?;

    let dm = match api.sql_family() {
        SqlFamily::Mysql => {
            indoc! {r##"
                model Post {
                    id      Int   @id @default(autoincrement())
                    user_id Int?
                    User    User? @relation("Post_user_idToUser", fields: [user_id], references: [id])
                    @@index([user_id], name: "user_id")
                }

                model User {
                    id   Int    @id @default(autoincrement())
                    Post Post[] @relation("Post_user_idToUser")
                }
            "##}
        }
        _ => {
            indoc! {r##"
                model Post {
                    id      Int   @id @default(autoincrement())
                    user_id Int?
                    User    User? @relation("Post_user_idToUser", fields: [user_id], references: [id])
                }

                model User {
                    id   Int    @id @default(autoincrement())
                    Post Post[] @relation("Post_user_idToUser")
                }
            "##}
        }
    };

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn default_values_on_relations(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER REFERENCES \"User\"(\"id\") Default 0");
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Post {
            id      Int   @id @default(autoincrement())
            user_id Int?  @default(0)
            User    User? @relation(fields: [user_id], references: [id])
        }

        model User {
            id   Int    @id @default(autoincrement())
            Post Post[]
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn prisma_1_0_relations(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::custom("char(25)").primary(true));
            });

            migration.create_table("Royalty", |t| {
                t.add_column("id", types::custom("char(25)").primary(true));
            });

            migration.create_table("_BookRoyalty", |t| {
                t.add_column("id", types::custom("char(25)").primary(true));
                t.add_column("A", types::custom("char(25)").nullable(false));
                t.add_column("B", types::custom("char(25)").nullable(false));

                t.add_foreign_key(&["A"], "Book", &["id"]);
                t.add_foreign_key(&["B"], "Royalty", &["id"]);

                t.add_index("double", types::index(vec!["A", "B"]).unique(true));
                t.add_index("single", types::index(vec!["B"]).unique(false));
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Book {
            id        String      @id
            Royalty   Royalty[]   @relation("BookRoyalty")
        }

        model Royalty {
            id        String      @id
            Book      Book[]      @relation("BookRoyalty")
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn relations_should_avoid_name_clashes(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("y", |t| {
                t.add_column("id", types::integer().primary(true));
                t.add_column("x", types::integer().nullable(false));
            });

            migration.create_table("x", |t| {
                t.add_column("id", types::integer().primary(true));
                t.add_column("y", types::integer().nullable(false));
                t.add_foreign_key(&["y"], "y", &["id"]);
            });
        })
        .await?;

    let dm = match api.sql_family() {
        SqlFamily::Sqlite => {
            indoc! {r##"
                model x {
                    id Int @id @default(autoincrement())
                    y  Int
                    y_xToy  y      @relation(fields: [y], references: [id])
                }

                model y {
                    id Int @id @default(autoincrement())
                    x  Int
                    x_xToy  x[]
                }
            "##}
        }
        SqlFamily::Mysql => {
            indoc! {r##"
                model x {
                    id Int @id
                    y  Int
                    y_xToy  y      @relation(fields: [y], references: [id])
                    @@index([y], name: "y")
                }

                model y {
                    id Int @id
                    x  Int
                    x_xToy  x[]
                }
            "##}
        }
        _ => {
            indoc! {r##"
                model x {
                    id Int @id
                    y  Int
                    y_xToy  y      @relation(fields: [y], references: [id])
                }

                model y {
                    id Int @id
                    x  Int
                    x_xToy  x[]
                }
            "##}
        }
    };

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

// SQL Server cannot form a foreign key without the related columns being part
// of a primary or candidate keys.
#[test_each_connector]
async fn relations_should_avoid_name_clashes_2(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("x", move |t| {
                t.add_column("id", types::primary());
                t.add_column("y", types::integer().nullable(false));
                t.add_index("unique_y_id", types::index(vec!["id", "y"]).unique(true));

                if sql_family.is_sqlite() {
                    t.add_foreign_key(&["y"], "y", &["id"]);
                }
            });

            migration.create_table("y", move |t| {
                t.add_column("id", types::primary());
                t.add_column("x", types::integer().nullable(false));
                t.add_column("fk_x_1", types::integer().nullable(false));
                t.add_column("fk_x_2", types::integer().nullable(false));

                if sql_family.is_sqlite() {
                    t.add_foreign_key(&["fk_x_1", "fk_x_2"], "x", &["id", "y"]);
                }
            });

            if !sql_family.is_sqlite() {
                migration.change_table("x", |t| {
                    t.add_foreign_key(&["y"], "y", &["id"]);
                });

                migration.change_table("y", |t| {
                    t.add_foreign_key(&["fk_x_1", "fk_x_2"], "x", &["id", "y"]);
                });
            }
        })
        .await?;

    let dm = match sql_family {
        SqlFamily::Mysql => {
            indoc! { r##"
                model x {
                    id                   Int @id @default(autoincrement())
                    y                    Int
                    y_x_yToy             y   @relation("x_yToy", fields: [y], references: [id])
                    y_xToy_fk_x_1_fk_x_2 y[] @relation("xToy_fk_x_1_fk_x_2")
                    @@unique([id, y], name: "unique_y_id")
                    @@index([y], name: "y")
                }

                model y {
                    id                   Int @id @default(autoincrement())
                    x                    Int
                    fk_x_1               Int
                    fk_x_2               Int
                    x_xToy_fk_x_1_fk_x_2 x   @relation("xToy_fk_x_1_fk_x_2", fields: [fk_x_1, fk_x_2], references: [id, y])
                    x_x_yToy             x[] @relation("x_yToy")
                    @@index([fk_x_1, fk_x_2], name: "fk_x_1")
                }
            "##}
        }
        _ => {
            indoc! { r##"
                model x {
                    id                   Int @id @default(autoincrement())
                    y                    Int
                    y_x_yToy             y   @relation("x_yToy", fields: [y], references: [id])
                    y_xToy_fk_x_1_fk_x_2 y[] @relation("xToy_fk_x_1_fk_x_2")
                    @@unique([id, y], name: "unique_y_id")
                }

                model y {
                    id                   Int @id @default(autoincrement())
                    x                    Int
                    fk_x_1               Int
                    fk_x_2               Int
                    x_xToy_fk_x_1_fk_x_2 x   @relation("xToy_fk_x_1_fk_x_2", fields: [fk_x_1, fk_x_2], references: [id, y])
                    x_x_yToy             x[] @relation("x_yToy")
                }
            "##}
        }
    };

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn one_to_many_relation_field_names_do_not_conflict_with_many_to_many_relation_field_names(
    api: &TestApi,
) -> crate::TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |table| {
                table.add_column("id", types::primary());
            });

            migration.create_table("Event", |table| {
                table.add_column("id", types::primary());
                table.add_column("host_id", types::integer().nullable(false));

                table.add_foreign_key(&["host_id"], "User", &["id"]);
            });

            migration.create_table("_EventToUser", |table| {
                table.add_column("A", types::integer().nullable(false));
                table.add_column("B", types::integer().nullable(false));

                table.add_foreign_key(&["A"], "Event", &["id"]);
                table.add_foreign_key(&["B"], "User", &["id"]);

                table.add_index(
                    "_EventToUser_AB_unique",
                    barrel::types::index(vec!["A", "B"]).unique(true),
                );

                table.add_index("_EventToUser_B_index", barrel::types::index(vec!["B"]));
            });
        })
        .await?;

    let extra_index = if sql_family.is_mysql() {
        r#"@@index([host_id], name: "host_id")"#
    } else {
        ""
    };

    let expected_dm = format!(
        r#"
            model Event {{
                id                           Int    @id @default(autoincrement())
                host_id                      Int
                User_EventToUser             User   @relation(fields: [host_id], references: [id])
                User_EventToUserManyToMany   User[] @relation("EventToUserManyToMany")
                {}
            }}

            model User {{
                id                           Int     @id @default(autoincrement())
                Event_EventToUser            Event[]
                Event_EventToUserManyToMany  Event[] @relation("EventToUserManyToMany")
            }}
        "#,
        extra_index
    );

    assert_eq_datamodels!(&expected_dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn many_to_many_relation_field_names_do_not_conflict_with_themselves(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |table| {
                table.add_column("id", barrel::types::primary());
            });

            migration.create_table("_Friendship", |table| {
                table.add_column("A", types::integer().nullable(false));
                table.add_column("B", types::integer().nullable(false));

                table.add_foreign_key(&["A"], "User", &["id"]);
                table.add_foreign_key(&["B"], "User", &["id"]);

                table.add_index(
                    "_FriendShip_AB_unique",
                    barrel::types::index(vec!["A", "B"]).unique(true),
                );
                table.add_index("_FriendShip_B_index", barrel::types::index(vec!["B"]));
            });

            migration.create_table("_Frenemyship", |table| {
                table.add_column("A", types::integer().nullable(false));
                table.add_column("B", types::integer().nullable(false));

                table.add_foreign_key(&["A"], "User", &["id"]);
                table.add_foreign_key(&["B"], "User", &["id"]);

                table.add_index(
                    "_Frenemyship_AB_unique",
                    barrel::types::index(vec!["A", "B"]).unique(true),
                );
                table.add_index("_Frenemyship_B_index", barrel::types::index(vec!["B"]));
            });
        })
        .await?;

    let expected_dm = indoc! {r#"
        model User {
            id                 Int    @id @default(autoincrement())
            User_B_Frenemyship User[] @relation("Frenemyship")
            User_A_Frenemyship User[] @relation("Frenemyship")
            User_B_Friendship  User[] @relation("Friendship")
            User_A_Friendship  User[] @relation("Friendship")
        }
    "#};

    assert_eq_datamodels!(expected_dm, &api.introspect().await?);

    Ok(())
}
