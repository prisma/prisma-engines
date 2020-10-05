use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_simple_table_with_gql_types_must_work(api: &TestApi) {
    let barrel = api.barrel();

    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("bool", types::boolean());
                t.add_column("float", types::float());
                t.add_column("date", types::custom("datetime2"));
                t.add_column("id", types::primary());
                t.add_column("int", types::integer());
                t.add_column("string", types::custom("nvarchar(max)"));
            });
            migration.create_table("_RelayId", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("stableModelIdentifier   int");
            });
        })
        .await;

    let dm = r#"
            model Blog {
                bool    Boolean
                float   Float
                date    DateTime
                id      Int @id @default(autoincrement())
                int     Int
                string  String
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_booleans_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom("bool1 bit");
                    t.inject_custom("bool2 bit default null");
                    t.inject_custom("bool3 bit default 1");
                    t.inject_custom("bool4 bit default 0");
                });
            },
            api.db_name(),
        )
        .await;
    let dm = r#"    
            model Blog {
              id       Int      @id @default(autoincrement())
              bool1    Boolean?
              bool2    Boolean?
              bool3    Boolean? @default(true)
              bool4    Boolean? @default(false)
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_table_with_compound_primary_keys_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::integer());
                    t.add_column("authorId", types::varchar(10));
                    t.inject_custom("PRIMARY KEY ([id], [authorId])");
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Blog {
                id Int
                authorId String
                @@id([id, authorId])
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_table_with_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("authorId", types::custom("nvarchar(10)"));
                    t.add_index("test", types::index(vec!["authorId"]).unique(true));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Blog {
                id      Int @id @default(autoincrement())
                authorId String @unique
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_table_with_multi_column_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("firstname", types::varchar(10));
                    t.add_column("lastname", types::varchar(10));
                    t.add_index("test", types::index(vec!["firstname", "lastname"]).unique(true));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                id      Int @id @default(autoincrement())
                firstname String
                lastname String
                @@unique([firstname, lastname], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_table_with_required_and_optional_columns_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("requiredname", types::custom("nvarchar(255)").nullable(false));
                    t.add_column("optionalname", types::custom("nvarchar(255)").nullable(true));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                id      Int @id @default(autoincrement())
                requiredname String
                optionalname String?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_table_with_default_values_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("a", types::text());
                    t.add_column("id", types::primary());
                    t.inject_custom("[bool] bit NOT NULL DEFAULT 0");
                    t.inject_custom("[bool2] bit NOT NULL DEFAULT 1");
                    t.inject_custom("[float] float(53) NOT NULL DEFAULT 5.3");
                    t.inject_custom("[int] int NOT NULL DEFAULT 5");
                    t.inject_custom("[string] nvarchar(4) NOT NULL DEFAULT 'Test'");
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                a String
                id      Int @id @default(autoincrement())
                bool Boolean @default(false)
                bool2 Boolean @default(true)
                float Float @default(5.3)
                int Int @default(5)
                string String @default("Test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_table_with_a_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("a", types::custom("nvarchar(10)"));
                    t.add_column("id", types::primary());
                    t.add_index("test", types::index(vec!["a"]));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                a String
                id      Int @id @default(autoincrement())
                @@index([a], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_table_with_a_multi_column_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("a", types::varchar(10));
                    t.add_column("b", types::varchar(10));
                    t.add_column("id", types::primary());
                    t.add_index("test", types::index(vec!["a", "b"]));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                a String
                b String
                id      Int @id @default(autoincrement())
                @@index([a,b], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//todo maybe need to split due to
// no function default values on mysql 5.7 and 8.0 -.-
// maria db allows this
#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_default_value_as_dbgenerated_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("string_static_char char(5) Default 'test'");
                t.inject_custom("string_static_char_null char(5) Default NULL");
                t.inject_custom("string_static_varchar varchar(5) Default 'test'");
                t.inject_custom("int_static int DEFAULT 2");
                t.inject_custom("float_static float(53) DEFAULT 1.43");
                t.inject_custom("boolean_static bit DEFAULT 1");
                t.inject_custom("datetime_now datetime2 NULL DEFAULT CURRENT_TIMESTAMP");
                t.inject_custom("datetime_now_lc datetime2 NULL DEFAULT current_timestamp");
            });
        })
        .await;

    let dm = r#"
            model Test {
                id                      Int                 @id @default(autoincrement())
                string_static_char      String?             @default("test")
                string_static_char_null String?     
                string_static_varchar   String?             @default("test") 
                int_static              Int?                @default(2)
                float_static            Float?              @default(1.43)                   
                boolean_static          Boolean?            @default(true)
                datetime_now            DateTime?           @default(now())
                datetime_now_lc         DateTime?           @default(now())
            }
        "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn introspecting_a_table_non_id_autoincrement_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Test", |t| {
                    t.inject_custom("id int primary key");
                    t.inject_custom("authorId int identity(1,1) unique");
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Test {
              id       Int @id
              authorId Int @default(autoincrement()) @unique
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
