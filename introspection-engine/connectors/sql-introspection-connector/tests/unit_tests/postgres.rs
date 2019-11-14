use crate::*;
use quaint::pool::SqlFamily;
use barrel::types;
use test_harness::*;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test]
fn introspecting_a_simple_table_with_gql_types_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("bool", types::boolean());
                t.add_column("float", types::float());
                t.add_column("date", types::date());
                t.add_column("id", types::primary());
                t.add_column("int", types::integer());
                t.add_column("string", types::text());
            });
        });
        let dm = r#"
            model Blog {
                bool    Boolean
                date    DateTime
                float   Float
                id      Int @id(strategy: NONE) @sequence(name: "Blog_id_seq", allocationSize: 1, initialValue: 1)
                int     Int 
                string  String
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_table_with_compound_primary_keys_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::integer());
                t.add_column("authorId", types::text());
                t.inject_custom("PRIMARY KEY (\"id\", \"authorId\")");
            });
        });

        let dm = r#"
            model Blog {
                authorId String
                id Int
                @@id([id, authorId])
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_table_with_unique_index_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("authorId", types::text());
            });
            migration.inject_custom("Create Unique Index \"test\" on \"introspection-engine\".\"Blog\"( \"authorId\")");
        });

        let dm = r#"
            model Blog {
                authorId String @unique
                id      Int @id(strategy: NONE) @sequence(name: "Blog_id_seq", allocationSize: 1, initialValue: 1)
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_table_with_multi_column_unique_index_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("firstname", types::text());
                t.add_column("lastname", types::text());
            });
            migration.inject_custom(
                "Create Unique Index \"test\" on \"introspection-engine\".\"User\"( \"firstname\", \"lastname\")",
            );
        });

        let dm = r#"
            model User {
                firstname String
                id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
                lastname String
                @@unique([firstname, lastname], name: "test")
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_table_with_required_and_optional_columns_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("requiredname", types::text().nullable(false));
                t.add_column("optionalname", types::text().nullable(true));
            });
        });

        let dm = r#"
            model User {
                id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
                optionalname String?
                requiredname String
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
#[ignore]
fn introspecting_a_table_with_datetime_default_values_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("\"joined\" date DEFAULT CURRENT_DATE")
            });
        });

        let dm = r#"
            model User {
                id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
                joined DateTime? @default(now())
                name String
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_table_with_default_values_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::text());
                t.add_column("id", types::primary());
                t.inject_custom("\"bool\" Boolean NOT NULL DEFAULT false");
                t.inject_custom("\"bool2\" Boolean NOT NULL DEFAULT 'off'");
                t.inject_custom("\"float\" Float NOT NULL DEFAULT 5.3");
                t.inject_custom("\"int\" INTEGER NOT NULL DEFAULT 5");
                t.inject_custom("\"string\" TEXT NOT NULL DEFAULT 'Test'");
            });
        });

        let dm = r#"
            model User {
                a String
                bool Boolean @default(false)
                bool2 Boolean @default(false)
                float Float @default(5.3)
                id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
                int Int @default(5)
                string String @default("Test")
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_table_with_a_non_unique_index_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::text());
                t.add_column("id", types::primary());
            });
            migration.inject_custom("Create Index \"test\" on \"introspection-engine\".\"User\"(\"a\")");
        });

        let dm = r#"
            model User {
                a String
                id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
                @@index([a], name: "test")
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_table_with_a_multi_column_non_unique_index_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::text());
                t.add_column("b", types::text());
                t.add_column("id", types::primary());
            });
            migration.inject_custom("Create Index \"test\" on \"introspection-engine\".\"User\"(\"a\",\"b\")");
        });

        let dm = r#"
            model User {
                a String
                b String
                id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
                @@index([a,b], name: "test")
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

//relations
#[test]
fn introspecting_a_one_to_one_req_relation_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER NOT NULL UNIQUE REFERENCES \"User\"(\"id\")")
            });
        });

        let dm = r#"
              model Post {
               id      Int @id(strategy: NONE) @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
               user_id User
            }
          
            model User {
               id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               post Post? 
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_two_one_to_one_relations_between_the_same_models_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER NOT NULL UNIQUE REFERENCES \"User\"(\"id\")")
            });
            migration.inject_custom("ALTER TABLE \"introspection-engine\".\"User\" ADD Column \"post_id\" INTEGER NOT NULL UNIQUE REFERENCES \"Post\"(\"id\")")
        });

        let dm = r#"
            model Post {
               id      Int @id(strategy: NONE) @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
               user_id User  @relation("Post_user_idToUser")
               user    User? @relation("PostToUser_post_id", references: [post_id])
            }
        
            model User {
               id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               post_id Post  @relation("PostToUser_post_id")
               post Post?    @relation("Post_user_idToUser")
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_one_to_one_relation_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER UNIQUE REFERENCES \"User\"(\"id\")");
            });
        });

        let dm = r#"        
            model Post {
               id      Int @id(strategy: NONE) @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
               user_id User?
            }
            
            model User {
               id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               post Post? 
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_one_to_many_relation_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER REFERENCES \"User\"(\"id\")");
            });
        });

        let dm = r#"  
            model Post {
               id      Int @id(strategy: NONE) @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
               user_id User?
            }
            
            model User {
               id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               posts Post[] 
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_one_req_to_many_relation_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_id INTEGER NOT NULL REFERENCES \"User\"(\"id\")");
            });
        });

        let dm = r#"
            model Post {
               id      Int @id(strategy: NONE) @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
               user_id User
            }
            
            model User {
               id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               posts Post[] 
            }
       "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_prisma_many_to_many_relation_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("_PostToUser", |t| {
                t.inject_custom(
                    "A INTEGER NOT NULL REFERENCES  \"Post\"(\"id\"),
                    B INTEGER NOT NULL REFERENCES  \"User\"(\"id\")",
                )
            });
            migration
                .inject_custom("CREATE UNIQUE INDEX test ON \"introspection-engine\".\"_PostToUser\" (\"a\", \"b\");")
        });

        let dm = r#"
            model Post {
               id      Int @id(strategy: NONE) @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
               users User[] 
            }
            
            model User {
               id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               posts Post[] 
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

// Todo
#[test]
fn introspecting_a_many_to_many_relation_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("PostsToUsers", |t| {
                t.inject_custom(
                    "user_id INTEGER NOT NULL REFERENCES  \"User\"(\"id\"),
                    post_id INTEGER NOT NULL REFERENCES  \"Post\"(\"id\")",
                )
            });
        });

        let dm = r#"
            model Post {
               id      Int @id(strategy: NONE) @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
               postsToUserses PostsToUsers[] @relation(references: [post_id])
            }

            model PostsToUsers {
              post_id Post 
              user_id User
            }
            
            model User {
               id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               postsToUserses PostsToUsers[] 
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

#[test]
fn introspecting_a_many_to_many_relation_with_extra_fields_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("PostsToUsers", |t| {
                t.inject_custom(
                    "date    date,
                          user_id INTEGER NOT NULL REFERENCES  \"User\"(\"id\"),
                    post_id INTEGER NOT NULL REFERENCES  \"Post\"(\"id\")",
                )
            });
        });

        let dm = r#"
            model Post {
               id      Int @id(strategy: NONE) @sequence(name: "Post_id_seq", allocationSize: 1, initialValue: 1)
               postsToUserses PostsToUsers[] @relation(references: [post_id])
            }
            
            model PostsToUsers {
              date    DateTime?
              post_id Post 
              user_id User
            }
            
            model User {
               id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
               postsToUserses PostsToUsers[] 
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}
#[test]
fn introspecting_a_self_relation_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom(
                    "recruited_by INTEGER  REFERENCES \"User\" (\"id\"),
                     direct_report INTEGER REFERENCES \"User\" (\"id\")",
                )
            });
        });

        let dm = r#"
            model User {
                direct_report                  User?  @relation("UserToUser_direct_report")
                id      Int @id(strategy: NONE) @sequence(name: "User_id_seq", allocationSize: 1, initialValue: 1)
                recruited_by                   User?  @relation("UserToUser_recruited_by")
                users_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
                users_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(&result, dm);
    });
}

// enums

// native arrays
