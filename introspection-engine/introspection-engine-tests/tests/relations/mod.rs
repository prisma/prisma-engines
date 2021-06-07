use barrel::types;
use indoc::formatdoc;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector]
async fn one_to_one_req_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_key", types::index(&["user_id"]).unique(true));
                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn one_to_one_relation_on_a_singular_primary_key(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().nullable(false));
                t.add_index("Post_id_key", types::index(&["id"]).unique(true));
                t.add_constraint(
                    "Post_id_fkey",
                    types::foreign_constraint(&["id"], "User", &["id"], None, None),
                );
            });
        })
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn two_one_to_one_relations_between_the_same_models(api: &TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("post_id", types::integer().nullable(false));
                t.add_index("User_post_id_key", types::index(&["post_id"]).unique(true));

                // Other databases can't create a foreign key before the table
                // exists, SQLite can, but cannot alter table with a foreign
                // key.
                if sql_family.is_sqlite() {
                    t.add_constraint(
                        "User_post_id_fkey",
                        types::foreign_constraint(&["post_id"], "Post", &["id"], None, None),
                    );
                }
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_key", types::index(&["user_id"]).unique(true));
                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });

            // Other databases can't create a foreign key before the table
            // exists, SQLite can, but cannot alter table with a foreign
            // key.
            if !sql_family.is_sqlite() {
                migration.change_table("User", |t| {
                    t.add_constraint(
                        "User_post_id_fkey",
                        types::foreign_constraint(&["post_id"], "Post", &["id"], None, None),
                    );
                })
            }
        })
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_one_to_one_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(true));
                t.add_index("Post_user_id_key", types::index(&["user_id"]).unique(true));
                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    // assert_eq!(true, false);
    Ok(())
}

#[test_connector]
async fn a_one_to_one_relation_referencing_non_id(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("email", types::varchar(10).nullable(true));
                t.add_index("User_email_key", types::index(&["email"]).unique(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_email", types::varchar(10).nullable(true));
                t.add_index("Post_user_email_key", types::index(&["user_email"]).unique(true));
                t.add_constraint(
                    "Post_user_email_fkey",
                    types::foreign_constraint(&["user_email"], "User", &["email"], None, None),
                );
            });
        })
        .await?;

    let native_type = if api.sql_family().is_sqlite() {
        ""
    } else {
        "@db.VarChar(10)"
    };

    let dm = formatdoc! {r##"
        model Post {{
            id         Int     @id @default(autoincrement())
            user_email String? @unique {}
            User       User?   @relation(fields: [user_email], references: [email])
        }}

        model User {{
            id    Int     @id @default(autoincrement())
            email String? @unique {}
            Post  Post?
        }}
    "##, native_type, native_type};

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_one_to_many_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().unique(false).nullable(true));
                t.add_index("Post_user_id_idx", types::index(&["user_id"]));
                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
        .await?;

    let dm = indoc! {r##"
                model Post {
                    id      Int   @id @default(autoincrement())
                    user_id Int?
                    User    User? @relation(fields: [user_id], references: [id])

                    @@index([user_id])
                }

                model User {
                    id   Int    @id @default(autoincrement())
                    Post Post[]
                }
            "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_one_req_to_many_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().unique(false).nullable(false));
                t.add_index("Post_user_id_idx", types::index(&["user_id"]));
                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
        .await?;

    let dm = indoc! {r##"
                model Post {
                    id      Int   @id @default(autoincrement())
                    user_id Int
                    User    User @relation(fields: [user_id], references: [id])
                    
                    @@index([user_id])
                }

                model User {
                    id   Int    @id @default(autoincrement())
                    Post Post[]
                }
            "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_prisma_many_to_many_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("_PostToUser", |t| {
                t.add_column("A", types::integer().nullable(false).unique(false));
                t.add_column("B", types::integer().nullable(false).unique(false));

                t.add_foreign_key(&["A"], "Post", &["id"]);
                t.add_foreign_key(&["B"], "User", &["id"]);

                t.add_index("test", types::index(vec!["A", "B"]).unique(true));
                t.add_index("test2", types::index(vec!["B"]).unique(false));
            });
        })
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_many_to_many_relation_with_an_id(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("PostsToUsers", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("PostsToUsers_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_column("post_id", types::integer().nullable(false));

                t.add_index("PostsToUsers_post_id_idx", types::index(&["post_id"]));
                t.add_index("PostsToUsers_user_id_idx", types::index(&["user_id"]));
                t.add_constraint(
                    "PostsToUsers_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
                t.add_constraint(
                    "PostsToUsers_post_id_fkey",
                    types::foreign_constraint(&["post_id"], "Post", &["id"], None, None),
                );
            });
        })
        .await?;

    let indices = if api.sql_family().is_sqlite() {
        "@@index([user_id])
         @@index([post_id])"
    } else {
        "@@index([post_id])
         @@index([user_id])"
    };

    let dm = format!(
        r#"
                model Post {{
                    id           Int            @id @default(autoincrement())
                    PostsToUsers PostsToUsers[]
                }}

                model PostsToUsers {{
                    id      Int  @id @default(autoincrement())
                    user_id Int
                    post_id Int
                    Post    Post @relation(fields: [post_id], references: [id])
                    User    User @relation(fields: [user_id], references: [id])
                    
                    {}
                }}

                model User {{
                    id           Int            @id @default(autoincrement())
                    PostsToUsers PostsToUsers[]
                }}
            "#,
        indices
    );

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_self_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("recruited_by", types::integer().nullable(true));
                t.add_column("direct_report", types::integer().nullable(true));

                t.add_index("User_direct_report_idx", types::index(&["direct_report"]));
                t.add_index("User_recruited_by_idx", types::index(&["recruited_by"]));
                t.add_constraint(
                    "User_recruited_by_fkey",
                    types::foreign_constraint(&["recruited_by"], "User", &["id"], None, None),
                );
                t.add_constraint(
                    "User_direct_report_fkey",
                    types::foreign_constraint(&["direct_report"], "User", &["id"], None, None),
                );
            });
        })
        .await?;

    let indices = if api.sql_family().is_sqlite() {
        "@@index([recruited_by])
         @@index([direct_report])"
    } else {
        "@@index([direct_report])
        @@index([recruited_by])"
    };

    let dm = format!(
        r##"
                model User {{
                    id                                  Int    @id @default(autoincrement())
                    recruited_by                        Int?
                    direct_report                       Int?
                    User_UserToUser_direct_report       User?  @relation("UserToUser_direct_report", fields: [direct_report], references: [id])
                    User_UserToUser_recruited_by        User?  @relation("UserToUser_recruited_by", fields: [recruited_by], references: [id])
                    other_User_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
                    other_User_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")
                    {}
                }}
            "##,
        indices
    );

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

// SQLite will always make the primary key autoincrement, which makes no sense to build.
#[test_connector(exclude(Sqlite))]
async fn id_fields_with_foreign_key(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });
            migration.create_table("Post", move |t| {
                t.add_column("user_id", types::integer().nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["user_id"]));
                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

//todo ignored for now since the order of the fks is unstable. Since we now care about the name this
//test is now unstable.
//We also need to think whether we want to randomly ignore one
// #[test_connector]
// async fn duplicate_fks_should_ignore_one_of_them(api: &TestApi) -> TestResult {
//     api.barrel()
//         .execute(|migration| {
//             migration.create_table("User", |t| {
//                 t.add_column("id", types::integer().increments(true).nullable(false));
//                 t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
//             });
//
//             migration.create_table("Post", |t| {
//                 t.add_column("id", types::integer().increments(true).nullable(false));
//                 t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
//                 t.add_column("user_id", types::integer().nullable(true));
//                 t.add_index("Post_user_id_idx", types::index(&["user_id"]));
//                 t.add_constraint(
//                     "Post_user_id_fkey",
//                     types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
//                 );
//                 t.add_constraint(
//                     "APost_user_id_fkey",
//                     types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
//                 );
//             });
//         })
//         .await?;
//
//     let dm = indoc! {r##"
//                 model Post {
//                     id      Int   @id @default(autoincrement())
//                     user_id Int?
//                     User    User? @relation(fields: [user_id], references: [id])
//                     @@index([user_id])
//                 }
//
//                 model User {
//                     id   Int    @id @default(autoincrement())
//                     Post Post[]
//                 }
//             "##
//     };
//
//     api.assert_eq_datamodels(dm, &api.introspect().await?);
//
//     Ok(())
// }

#[test_connector(tags(Postgres))]
async fn default_values_on_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(exclude(Mssql))]
async fn prisma_1_0_relations(api: &TestApi) -> TestResult {
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

    let native_string = if !api.sql_family().is_sqlite() {
        "@db.Char(25)"
    } else {
        ""
    };

    let dm = formatdoc! {r##"
        model Book {{
            id        String      @id {}
            Royalty   Royalty[]   @relation("BookRoyalty")
        }}

        model Royalty {{
            id        String      @id {}
            Book      Book[]      @relation("BookRoyalty")
        }}
    "##, native_string, native_string};

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn relations_should_avoid_name_clashes(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("y", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("y_pkey", types::primary_constraint(&["id"]));
                t.add_column("x", types::integer().nullable(false));
            });

            migration.create_table("x", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("x_pkey", types::primary_constraint(&["id"]));
                t.add_column("y", types::integer().nullable(false));
                t.add_index("x_y_idx", types::index(&["y"]));
                t.add_constraint("x_y_fkey", types::foreign_constraint(&["y"], "y", &["id"], None, None));
            });
        })
        .await?;

    let dm = indoc! {r##"
                model x {
                    id Int @id @default(autoincrement())
                    y  Int
                    y_xToy  y      @relation(fields: [y], references: [id])
                    @@index([y])
                }

                model y {
                    id Int @id @default(autoincrement())
                    x  Int
                    x_xToy  x[]
                }
            "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

// SQL Server cannot form a foreign key without the related columns being part
// of a primary or candidate keys.
#[test_connector]
async fn relations_should_avoid_name_clashes_2(api: &TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("x", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("x_pkey", types::primary_constraint(&["id"]));
                t.add_column("y", types::integer().nullable(false));
                t.add_index("x_id_y_key", types::index(vec!["id", "y"]).unique(true));
                t.add_index("x_y_idx", types::index(&["y"]));

                if sql_family.is_sqlite() {
                    t.add_constraint("x_y_fkey", types::foreign_constraint(&["y"], "y", &["id"], None, None));
                }
            });

            migration.create_table("y", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("y_pkey", types::primary_constraint(&["id"]));
                t.add_column("x", types::integer().nullable(false));
                t.add_column("fk_x_1", types::integer().nullable(false));
                t.add_column("fk_x_2", types::integer().nullable(false));
                t.add_index("y_fk_x_1_fk_x_2_idx", types::index(&["fk_x_1", "fk_x_2"]));

                t.add_constraint(
                    "y_fk_x_1_fk_x_2_fkey",
                    types::foreign_constraint(&["fk_x_1", "fk_x_2"], "x", &["id", "y"], None, None),
                );
            });

            if !sql_family.is_sqlite() {
                migration.change_table("x", |t| {
                    t.add_constraint("x_y_fkey", types::foreign_constraint(&["y"], "y", &["id"], None, None));
                });
            }
        })
        .await?;

    let dm = indoc! { r##"
                model x {
                    id                   Int @id @default(autoincrement())
                    y                    Int
                    y_x_yToy             y   @relation("x_yToy", fields: [y], references: [id])
                    y_xToy_fk_x_1_fk_x_2 y[] @relation("xToy_fk_x_1_fk_x_2")
                    @@unique([id, y])
                    @@index([y])
                }

                model y {
                    id                   Int @id @default(autoincrement())
                    x                    Int
                    fk_x_1               Int
                    fk_x_2               Int
                    x_xToy_fk_x_1_fk_x_2 x   @relation("xToy_fk_x_1_fk_x_2", fields: [fk_x_1, fk_x_2], references: [id, y])
                    x_x_yToy             x[] @relation("x_yToy")
                    @@index([fk_x_1, fk_x_2])
                }
            "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn one_to_many_relation_field_names_do_not_conflict_with_many_to_many_relation_field_names(
    api: &TestApi,
) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Event", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Event_pkey", types::primary_constraint(&["id"]));
                t.add_column("host_id", types::integer().nullable(false));

                t.add_index("Event_host_id_idx", types::index(&["host_id"]));
                t.add_constraint(
                    "Event_host_id_fkey",
                    types::foreign_constraint(&["host_id"], "User", &["id"], None, None),
                );
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

    let expected_dm = r#"
                model Event {
                    id                           Int    @id @default(autoincrement())
                    host_id                      Int
                    User_EventToUser             User   @relation(fields: [host_id], references: [id])
                    User_EventToUserManyToMany   User[] @relation("EventToUserManyToMany")
    
                    @@index([host_id])
                }
    
                model User {
                    id                           Int     @id @default(autoincrement())
                    Event_EventToUser            Event[]
                    Event_EventToUserManyToMany  Event[] @relation("EventToUserManyToMany")
                }
        "#;

    api.assert_eq_datamodels(&expected_dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn many_to_many_relation_field_names_do_not_conflict_with_themselves(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
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

    api.assert_eq_datamodels(expected_dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite))]
async fn one_to_one_req_relation_with_custom_fk_name(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_key", types::index(&["user_id"]).unique(true));
                t.add_constraint(
                    "CustomFKName",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Post {
            id       Int @id @default(autoincrement())
            user_id  Int  @unique
            User     User @relation(fields: [user_id], references: [id], map: "CustomFKName")
        }

        model User {
            id      Int @id @default(autoincrement())
            Post Post?
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}
