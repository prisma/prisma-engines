use barrel::types;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector]
async fn compound_foreign_keys_for_one_to_one_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());
                t.add_index("User_id_age_key", types::index(vec!["id", "age"]).unique(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));
                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
                t.add_index(
                    "Post_user_id_user_age_key",
                    types::index(vec!["user_id", "user_age"]).unique(true),
                );
            });
        })
        .await?;

    let dm = r#"
        model Post {
            id       Int   @id @default(autoincrement())
            user_id  Int?
            user_age Int?
            User     User? @relation(fields: [user_id, user_age], references: [id, age])

            @@unique([user_id, user_age])
        }

        model User {
            id   Int   @id @default(autoincrement())
            age  Int
            Post Post?

            @@unique([id, age])
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn compound_foreign_keys_for_required_one_to_one_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());
                t.add_index("User_id_age_key", types::index(vec!["id", "age"]).unique(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
                t.add_index(
                    "Post_user_id_user_age_key",
                    types::index(vec!["user_id", "user_age"]).unique(true),
                );
            });
        })
        .await?;

    let dm = r#"
        model Post {
            id       Int  @id @default(autoincrement())
            user_id  Int
            user_age Int
            User     User @relation(fields: [user_id, user_age], references: [id, age])

            @@unique([user_id, user_age])
        }

        model User {
            id   Int   @id @default(autoincrement())
            age  Int
            Post Post?

            @@unique([id, age])
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn compound_foreign_keys_for_one_to_many_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());
                t.add_index("User_id_age_key", types::index(vec!["id", "age"]).unique(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));
                t.add_index("Post_user_id_user_age_idx", types::index(vec!["user_id", "user_age"]));
                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
            });
        })
        .await?;

    let dm = r#"
        model Post {
            id       Int   @id @default(autoincrement())
            user_id  Int?
            user_age Int?
            User     User? @relation(fields: [user_id, user_age], references: [id, age])
            
            @@index([user_id, user_age])
        }

        model User {
            id   Int    @id @default(autoincrement())
            age  Int
            Post Post[]

            @@unique([id, age])
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn compound_foreign_keys_for_one_to_many_relations_with_mixed_requiredness(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());
                t.add_index("User_unique", types::index(vec!["id", "age"]).unique(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_column("user_age", types::integer().nullable(true));
                t.add_index("Post_index", types::index(vec!["user_id", "user_age"]));

                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
            });
        })
        .await?;

    let dm = r#"
        model Post {
            id       Int   @id @default(autoincrement())
            user_id  Int
            user_age Int?
            User     User? @relation(fields: [user_id, user_age], references: [id, age])
            
            @@index([user_id, user_age], map: "Post_index")
        }

        model User {
            id   Int    @id @default(autoincrement())
            age  Int
            Post Post[]

            @@unique([id, age], map: "User_unique")
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn compound_foreign_keys_for_required_one_to_many_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());
                t.add_index("user_unique", types::index(vec!["id", "age"]).unique(true));
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.add_index("Post_user_id_user_age_idx", types::index(&["user_id", "user_age"]));
                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
            });
        })
        .await?;

    let dm = r#"
        model Post {
            id       Int  @id @default(autoincrement())
            user_id  Int
            user_age Int
            User     User @relation(fields: [user_id, user_age], references: [id, age])
            @@index([user_id, user_age])
        }

        model User {
            id   Int    @id @default(autoincrement())
            age  Int
            Post Post[]

            @@unique([id, age], map: "user_unique")
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite))]
async fn compound_foreign_keys_for_required_self_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Person_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer());
                t.add_column("partner_age", types::integer());
                t.add_index(
                    "Person_partner_id_partner_age_idx",
                    types::index(&["partner_id", "partner_age"]),
                );
                t.add_constraint("Person_id_age_key", types::unique_constraint(vec!["id", "age"]));
                t.add_constraint(
                    "Person_partner_id_partner_age_fkey",
                    types::foreign_constraint(&["partner_id", "partner_age"], "Person", &["id", "age"], None, None),
                );
            });
        })
        .await?;

    let dm = r#"
        model Person {
            id           Int      @id @default(autoincrement())
            age          Int
            partner_id   Int
            partner_age  Int
            Person       Person   @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age])
            other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")

            @@unique([id, age])
            @@index([partner_id, partner_age])
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite))]
//just excluded due to test setup problems
async fn compound_foreign_keys_for_self_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Person_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer().nullable(true));
                t.add_column("partner_age", types::integer().nullable(true));
                t.add_constraint("Person_id_age_key", types::unique_constraint(vec!["id", "age"]));
                t.add_index(
                    "Person_partner_id_partner_age_idx",
                    types::index(vec!["partner_id", "partner_age"]),
                );
                t.add_constraint(
                    "Person_partner_id_partner_age_fkey",
                    types::foreign_constraint(&["partner_id", "partner_age"], "Person", &["id", "age"], None, None),
                );
            });
        })
        .await?;

    let dm = r#"
        model Person {
            id           Int      @id @default(autoincrement())
            age          Int
            partner_id   Int?
            partner_age  Int?
            Person       Person?  @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age])
            other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")

            @@unique([id, age])
            @@index([partner_id, partner_age])
    }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite))]
// excluded due to test setup problems around uniques and fks on sqlite
async fn compound_foreign_keys_with_defaults(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Person_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer().default(0));
                t.add_column("partner_age", types::integer().default(0));
                t.add_index(
                    "Person_partner_id_partner_age_idx",
                    types::index(vec!["partner_id", "partner_age"]).unique(false),
                );
                t.add_constraint("Person_id_age_key", types::unique_constraint(vec!["id", "age"]));
                t.add_constraint(
                    "Person_partner_id_partner_age_fkey",
                    types::foreign_constraint(&["partner_id", "partner_age"], "Person", &["id", "age"], None, None),
                );
            });
        })
        .await?;

    let dm = r#"
        model Person {
            id           Int      @id @default(autoincrement())
            age          Int
            partner_id   Int      @default(0)
            partner_age  Int      @default(0)
            Person       Person   @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age])
            other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")

            @@unique([id, age])
            @@index([partner_id, partner_age])   
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn compound_foreign_keys_for_one_to_many_relations_with_non_unique_index(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());

                t.add_index("User_id_age_key", types::index(vec!["id", "age"]).unique(true));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.add_index("Post_user_id_user_age_idx", types::index(&["user_id", "user_age"]));
                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
            });
        })
        .await?;

    let dm = r#"
        model Post {
            id       Int  @id @default(autoincrement())
            user_id  Int
            user_age Int
            User     User @relation(fields: [user_id, user_age], references: [id, age])
            
            @@index([user_id, user_age])
        }

        model User {
            id   Int    @id @default(autoincrement())
            age  Int
            Post Post[]

            @@unique([id, age])
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn repro_matt_references_on_wrong_side(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.add_constraint("a_pkey", types::primary_constraint(&["one", "two"]));
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("b_pkey", types::primary_constraint(&["id"]));
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.add_index("b_one_two_idx", types::index(&["one", "two"]));
                t.add_constraint(
                    "b_one_two_fkey",
                    types::foreign_constraint(&["one", "two"], "a", &["one", "two"], None, None),
                )
            });
        })
        .await?;

    let dm = r#"
        model a {
            one Int
            two Int
            b   b[]

            @@id([one, two])
        }

        model b {
            id  Int @id @default(autoincrement())
            one Int
            two Int

            a   a   @relation(fields: [one, two], references: [one, two])
            @@index([one, two])
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_compound_fk_pk_with_overlapping_primary_key(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.add_constraint("a_pkey", types::primary_constraint(&["one", "two"]));
            });
            migration.create_table("b", |t| {
                t.add_column("dummy", types::integer().nullable(false));
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.add_index("b_one_two_idx", types::index(&["one", "two"]));
                t.add_constraint(
                    "b_one_two_fkey",
                    types::foreign_constraint(&["one", "two"], "a", &["one", "two"], None, None),
                );
                t.add_constraint("b_pkey", types::primary_constraint(&["dummy", "one", "two"]));
            });
        })
        .await?;

    let dm = r#"
        model a {
            one Int
            two Int
            b   b[]

            @@id([one, two])
        }

        model b {
            dummy Int
            one   Int
            two   Int
            a     a   @relation(fields: [one, two], references: [one, two])

            @@id([dummy, one, two])
            @@index([one, two])
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

//todo sqlite order
#[test_connector(exclude(Sqlite))]
async fn compound_foreign_keys_for_duplicate_one_to_many_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
                t.add_column("age", types::integer());
                t.add_index("User_id_age_key", types::index(&["id", "age"]).unique(true));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));
                t.add_column("other_user_id", types::integer().nullable(true));
                t.add_column("other_user_age", types::integer().nullable(true));
                t.add_index(
                    "Post_other_user_id_other_user_age_idx",
                    types::index(&["other_user_id", "other_user_age"]),
                );
                t.add_index("Post_user_id_user_age_idx", types::index(&["user_id", "user_age"]));
                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
                t.add_constraint(
                    "Post_other_user_id_other_user_age_fkey",
                    types::foreign_constraint(&["other_user_id", "other_user_age"], "User", &["id", "age"], None, None),
                );
            });
        })
        .await?;

    let dm = r#"
        model Post {
            id                                               Int   @id @default(autoincrement())
            user_id                                          Int?
            user_age                                         Int?
            other_user_id                                    Int?
            other_user_age                                   Int?
            User_Post_other_user_id_other_user_ageToUser     User? @relation("Post_other_user_id_other_user_ageToUser", fields: [other_user_id, other_user_age], references: [id, age])
            User_Post_user_id_user_ageToUser                 User? @relation("Post_user_id_user_ageToUser", fields: [user_id, user_age], references: [id, age])
            
            @@index([other_user_id, other_user_age])
            @@index([user_id, user_age])
        }

        model User {
            id                                              Int    @id @default(autoincrement())
            age                                             Int
            Post_Post_other_user_id_other_user_ageToUser    Post[] @relation("Post_other_user_id_other_user_ageToUser")
            Post_Post_user_id_user_ageToUser                Post[] @relation("Post_user_id_user_ageToUser")

            @@unique([id, age])
        }
    "#;

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}
