mod test_harness;

use barrel::types;
use test_harness::*;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test]
fn introspecting_a_simple_table_with_gql_types_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Postgres, SqlFamily::Mysql], |test_setup, barrel| {
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
                id      Int @id
                int     Int 
                string  String
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}

#[test]
fn introspecting_a_table_with_compound_primary_keys_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Postgres, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::integer());
                t.add_column("authorId", types::text());

                // Simulate how we create primary keys in the migrations engine.
                t.inject_custom("PRIMARY KEY (\"id\", \"authorId\")");
            });
        });

        let dm = r#"
            model Blog {
                authorId String
                id Int
                @@id([id, authorId])
                @@unique([id, authorId], name: "sqlite_autoindex_Blog_1")
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}

#[test]
fn introspecting_a_table_with_unique_index_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Postgres, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("authorId", types::text());

                // Simulate how we create primary keys in the migrations engine.
            });
            migration.inject_custom("Create Unique Index \"introspection-engine\".\"test\" on \"Blog\"( \"authorId\")");
        });

        let dm = r#"
            model Blog {
                authorId String @unique
                id Int @id
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}

#[test]
fn introspecting_a_table_with_multi_column_unique_index_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Postgres, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("firstname", types::text());
                t.add_column("lastname", types::text());

                // Simulate how we create primary keys in the migrations engine.
            });
            migration.inject_custom(
                "Create Unique Index \"introspection-engine\".\"test\" on \"User\"( \"firstname\", \"lastname\")",
            );
        });

        let dm = r#"
            model User {
                firstname String
                id Int @id
                lastname String
                @@unique([firstname, lastname], name: "test")
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}

#[test]
fn introspecting_a_table_with_required_and_optional_columns_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Postgres, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("requiredname", types::text().nullable(false));
                t.add_column("optionalname", types::text().nullable(true));
            });
        });

        let dm = r#"
            model User {
                id Int @id
                optionalname String?
                requiredname String
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}

#[test]
fn introspecting_a_table_with_datetime_default_values_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Postgres, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::text());
                t.inject_custom("\"joined\" TIMESTAMP DEFAULT CURRENT_TIMESTAMP")
            });
        });

        let dm = r#"
            model User {
                id Int @id
                joined DateTime? @default(now())
                name String
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}

#[test]
fn introspecting_a_table_with_default_values_should_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Postgres, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("\"int\" INTEGER NOT NULL DEFAULT \"5\"");
                t.inject_custom("\"string\" TEXT NOT NULL DEFAULT \"\"");
            });
        });

        let dm = r#"
            model User {
                id Int @id
                int Int @default(5)
                string String @default("")
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}

// default values
// index
// multicolumn index
// sequence
// unique with index
// unique without index

// 1:1
// 1:1!
// 1:M
// 1!:M
// self
// duplicate self
// M:N prisma
// M:N other
// M:N extra fields
