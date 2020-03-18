use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("postgres"))]
#[test]
async fn compound_foreign_keys_should_work_for_required_one_to_one_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_name", types::text());
                t.inject_custom(
                    "FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User\"(\"id\", \"name\")",
                );
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(\"user_id\", \"user_name\")");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                 @id @default(autoincrement())
                user    User                @map(["user_id", "user_name"]) @relation(references:[id, name])
            }

            model User {
               id       Int                 @id @default(autoincrement())
               name     String
               post     Post?

               @@unique([id, name], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
#[test]
async fn compound_foreign_keys_should_work_for_one_to_one_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_name", types::text().nullable(true));
                t.inject_custom(
                    "FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User\"(\"id\", \"name\")",
                );
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(\"user_id\", \"user_name\")");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                 @id @default(autoincrement())
                user    User?                @map(["user_id", "user_name"]) @relation(references:[id, name])
            }

            model User {
               id       Int                 @id @default(autoincrement())
               name     String
               post     Post?

               @@unique([id, name], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
#[test]
async fn compound_foreign_keys_should_work_for_one_to_many_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_name", types::text().nullable(true));
                t.inject_custom(
                    "FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User\"(\"id\", \"name\")",
                );
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                 @id @default(autoincrement())
                user    User?                @map(["user_id", "user_name"]) @relation(references:[id, name])
            }

            model User {
               id       Int                 @id @default(autoincrement())
               name     String
               post     Post[]

               @@unique([id, name], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
#[test]
async fn compound_foreign_keys_should_work_for_required_one_to_many_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_name", types::text());
                t.inject_custom(
                    "FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User\"(\"id\", \"name\")",
                );
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                 @id @default(autoincrement())
                user    User                @map(["user_id", "user_name"]) @relation(references:[id, name])
            }

            model User {
               id       Int                 @id @default(autoincrement())
               name     String
               post     Post[]

               @@unique([id, name], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
#[test]
async fn compound_foreign_keys_should_work_for_self_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Person", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.add_column("partner_id", types::integer());
                t.add_column("partner_name", types::text());
                t.inject_custom(
                    "FOREIGN KEY (\"partner_id\",\"partner_name\") REFERENCES \"Person\"(\"id\", \"name\")",
                );
                t.inject_custom("CONSTRAINT \"person_unique\" UNIQUE (\"id\", \"name\")");
            });
        })
        .await;

    let dm = r#"
            model Person {
               id       Int         @id @default(autoincrement())
               name     String
               person   Person      @map(["partner_id", "partner_name"]) @relation("PersonToPerson_partner_id_partner_name", references: [id,name])
               other_person   Person[]    @relation("PersonToPerson_partner_id_partner_name")

               @@unique([id, name], name: "person_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
#[test]
async fn compound_foreign_keys_should_work_with_defaults(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Person", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.add_column("partner_id", types::integer().default(0));
                t.add_column("partner_name", types::text().default(""));
                t.inject_custom(
                    "FOREIGN KEY (\"partner_id\",\"partner_name\") REFERENCES \"Person\"(\"id\", \"name\")",
                );
                t.inject_custom("CONSTRAINT \"person_unique\" UNIQUE (\"id\", \"name\")");
            });
        })
        .await;

    let dm = r#"
            model Person {
               id       Int         @id @default(autoincrement())
               name     String
               person   Person      @map(["partner_id", "partner_name"]) @relation("PersonToPerson_partner_id_partner_name", references: [id, name])
               other_person  Person[]    @relation("PersonToPerson_partner_id_partner_name")

               @@unique([id, name], name: "person_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//todo decide on this,
// this can at most be a one:one relation, but with a more limited subset of available connections
// fetch this from indexes
// what about separate uniques? all @unique == @@unique ?? No! separate ones do not fully work since you can only connect to a subset of the @@unique case
// model.indexes contains a multi-field unique index that matches the colums exactly, then it is unique
// if there are separate uniques it probably should not become a relation
// what breaks by having an @@unique that refers to fields that do not have a representation on the model anymore due to the merged relation field?
//#[test_each_connector(tags("postgres"))]
//#[test]
//async fn compound_foreign_keys_should_work_for_one_to_one_relations_with_separate_uniques(api: &TestApi) {
//    let barrel = api.barrel();
//    let _setup_schema = barrel
//        .execute(|migration| {
//            migration.create_table("User", |t| {
//                t.add_column("id", types::primary());
//                t.add_column("name", types::text());
//                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"name\")");
//            });
//            migration.create_table("Post", |t| {
//                t.add_column("id", types::primary());
//                t.add_column("user_id", types::integer().unique(true));
//                t.add_column("user_name", types::text().unique(false));
//                t.inject_custom("FOREIGN KEY (\"user_id\",\"user_name\") REFERENCES \"User\"(\"id\", \"name\")");
//            });
//        })
//        .await;
//
//    let dm = r#"
//            model Post {
//                id      Int                 @id @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
//                user    User                @map(["user_id", "user_name"]) @relation(references:[id, name])
//            }
//
//            model User {
//               id       Int                 @id @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
//               name     String
//               post     Post?
//
//               @@unique([id, name], name: "user_unique")
//            }
//        "#;
//    let result = dbg!(api.introspect().await);
//    custom_assert(&result, dm);
//}

#[test_each_connector(tags("postgres"))]
#[test]
async fn compound_foreign_keys_should_work_for_one_to_many_relations_with_non_unique_index(
    api: &TestApi,
) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(\"id\", \"age\")");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.inject_custom(
                    "FOREIGN KEY (\"user_id\",\"user_age\") REFERENCES \"User\"(\"id\", \"age\")",
                );
                t.add_index("test", types::index(vec!["user_id", "user_age"]));
            });
        })
        .await;

    let dm = r#"
            model Post {
                id      Int                @id @default(autoincrement())
                user    User               @map(["user_id", "user_age"]) @relation(references:[id, age])

                @@index(user, name: "test")
            }

            model User {
               age      Int
               id       Int                @id @default(autoincrement())
               post     Post[]

               @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
#[test]
async fn repro_matt_references_on_wrong_side(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.inject_custom("Primary Key (\"one\", \"two\")");
            });
            migration.create_table("b", |t| {
                t.add_column("id", types::primary());
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.inject_custom("Foreign Key (\"one\", \"two\") references a(\"one\", \"two\")");
            });
        })
        .await;

    let dm = r#"
            model a {
              one Int
              two Int
              b   b[]
                    
              @@id([one, two])
            }
            
            model b {
              id Int @id  @default(autoincrement())
              a  a   @map(["one", "two"])
            }
              
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//todo referencing unknown field in index due to compound, will be fixed in follow up PR
// #[test_each_connector(tags("postgres"))]
// #[test]
// async fn compound_fk_pk(api: &TestApi) {
//     let barrel = api.barrel();
//     let _setup_schema = barrel
//         .execute(|migration| {
//             migration.create_table("a", |t| {
//                 t.add_column("one", types::integer().nullable(false));
//                 t.add_column("two", types::integer().nullable(false));
//                 t.inject_custom("Primary Key (\"one\", \"two\")");
//             });
//             migration.create_table("b", |t| {
//                 t.add_column("dummy", types::integer().nullable(false));
//                 t.add_column("one", types::integer().nullable(false));
//                 t.add_column("two", types::integer().nullable(false));
//                 t.inject_custom("Foreign Key (\"one\", \"two\") references a(\"one\", \"two\")");
//                 t.inject_custom("Primary Key (\"dummy\",\"one\", \"two\")");
//             });
//         })
//         .await;
//
//     let dm = r#"
//             model a {
//               one Int
//               two Int
//               b   b[]
//
//               @@id([one, two])
//             }
//
//             model b {
//               dummy Int
//               a     a   @map(["one", "two"])
//
//               @@id([dummy, one, two])
//             }
//         "#;
//     let result = dbg!(api.introspect().await);
//     custom_assert(&result, dm);
// }
