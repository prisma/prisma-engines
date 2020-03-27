use crate::*;
use barrel::types;
use pretty_assertions::assert_eq;
use test_harness::*;

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_simple_table_with_gql_types_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("bool", types::boolean());
                t.add_column("float", types::float());
                t.add_column("date", types::date());
                t.add_column("id", types::primary());
                t.add_column("int", types::integer());
                t.add_column("string", types::text());
            });
        })
        .await;

    let dm = r#"
            model Blog {
                bool    Boolean
                date    DateTime
                float   Float
                id      Int @id @default(autoincrement())
                int     Int
                string  String
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_serial_type_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.inject_custom("id serial not null primary key");
            });
        })
        .await;

    let dm = r#"
            model Blog {
                id      Int @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_compound_primary_keys_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::integer());
                t.add_column("authorId", types::text());
                t.inject_custom("PRIMARY KEY (\"id\", \"authorId\")");
            });
        })
        .await;
    let dm = r#"
            model Blog {
                authorId String
                id Int
                @@id([id, authorId])
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("authorId", types::text());
                t.add_index("test", types::index(vec!["authorId"]).unique(true));
            });
        })
        .await;

    let dm = r#"
            model Blog {
                authorId String @unique
                id      Int @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_multi_column_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("firstname", types::text());
                t.add_column("lastname", types::text());
                t.add_index("test", types::index(vec!["firstname", "lastname"]).unique(true));
            });
        })
        .await;

    let dm = r#"
            model User {
                firstname String
                id      Int @id @default(autoincrement())
                lastname String
                @@unique([firstname, lastname], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_required_and_optional_columns_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("requiredname", types::text().nullable(false));
                t.add_column("optionalname", types::text().nullable(true));
            });
        })
        .await;
    let dm = r#"
            model User {
                id      Int @id @default(autoincrement())
                optionalname String?
                requiredname String
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// #[test_each_connector(tags("postgres"))]
// async fn introspecting_a_table_with_datetime_default_values_should_work2(api: &TestApi) {
//     let barrel = api.barrel();
//     let _setup_schema = barrel
//         .execute(|migration| {
//             migration.create_table("User", |t| {
//                 t.add_column("id", types::primary());
//                 t.add_column("name", types::text());
//                 t.inject_custom("\"current_timestamp\" Timestamp with time zone DEFAULT CURRENT_TIMESTAMP");
//                 t.inject_custom("\"now\" Timestamp with time zone DEFAULT NOW()");
//             });
//         })
//         .await;
//     let dm = r#"
//             model User {
//                 id                  Int       @default(autoincrement()) @id
//                 current_timestamp   DateTime? @default(now())
//                 now                 DateTime? @default(now())
//                 name                String
//             }
//         "#;
//     let result = dbg!(api.introspect().await);
//     custom_assert(&result, dm);
// }

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_default_values_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::text());
                t.add_column("id", types::primary());
                t.inject_custom("\"bool\" Boolean NOT NULL DEFAULT false");
                t.inject_custom("\"bool2\" Boolean NOT NULL DEFAULT 'off'");
                t.inject_custom("\"float\" Float NOT NULL DEFAULT 5.3");
                t.inject_custom("\"int\" INTEGER NOT NULL DEFAULT 5");
                t.inject_custom("\"string\" TEXT NOT NULL DEFAULT 'Test'");
            });
        })
        .await;
    let dm = r#"
            model User {
                a String
                bool Boolean @default(false)
                bool2 Boolean @default(false)
                float Float @default(5.3)
                id      Int @id @default(autoincrement())
                int Int @default(5)
                string String @default("Test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_a_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::text());
                t.add_column("id", types::primary());
                t.add_index("test", types::index(vec!["a"]));
            });
        })
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

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_a_multi_column_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::text());
                t.add_column("b", types::text());
                t.add_column("id", types::primary());
                t.add_index("test", types::index(vec!["a", "b"]));
            });
        })
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

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_without_uniques_should_comment_it_out(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("user_id", types::foreign("User", "id").nullable(false).unique(false));
            });
        })
        .await;

    let dm = "// The underlying table does not contain a unique identifier and can therefore currently not be handled.\n// model Post {\n  // id      Int\n  // user_id Int\n  // User    User @relation(fields: [user_id], references: [id])\n// }\n\nmodel User {\n  id Int @default(autoincrement()) @id\n}";

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_default_values_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("numeric_int2 int2 Default 2");
                t.inject_custom("numeric_int4 int4 Default 4");
                t.inject_custom("numeric_int8 int8 Default 8");
                t.inject_custom("numeric_decimal decimal(8,4) Default 1234.1234");
                t.inject_custom("numeric_float4 float4 Default 123.1234");
                t.inject_custom("numeric_float8 float8 Default 123.1234");

                // numeric_serial2 serial2,
                // numeric_serial4 serial4,
                // numeric_serial8 serial8,
                // t.inject_custom("numeric_money money Default 123.12");
                // t.inject_custom("numeric_oid oid Default 42");

                t.inject_custom("string_char char(8) Default 'abcdefgh'");
                t.inject_custom("string_varchar varchar(8) Default 'abcd'");
                t.inject_custom("string_text text Default 'abcdefgh'");

                // binary_bytea bytea,
                // binary_bits  bit(80),
                // binary_bits_varying bit varying(80),
                // binary_uuid uuid,

                t.inject_custom("time_timestamp timestamp Default Now()");
                t.inject_custom("time_timestamptz timestamptz Default Now()");
                t.inject_custom("time_date date Default CURRENT_DATE"); //todo not recognized yet
                t.inject_custom("time_time time Default Now()");

                // time_timetz timetz,
                // time_interval interval,

                t.inject_custom("boolean_boolean boolean Default false");

                // network_cidr cidr,
                // network_inet inet,
                // network_mac  macaddr,
                // search_tsvector tsvector,
                // search_tsquery tsquery,
                // json_json json,
                // json_jsonb jsonb,
                // range_int4range int4range,
                // range_int8range int8range,
                // range_numrange numrange,
                // range_tsrange tsrange,
                // range_tstzrange tstzrange,
                // range_daterange daterange
            });
        })
        .await;

    let dm = r#"
            model Test {
                boolean_boolean     Boolean?        @default(false)
                id                  Int         @id @default(autoincrement())
                numeric_decimal     Float?          @default(1234.1234) 
                numeric_float4      Float?          @default(123.1234)
                numeric_float8      Float?          @default(123.1234)
                numeric_int2        Int?            @default(2)
                numeric_int4        Int?            @default(4)
                numeric_int8        Int?            @default(8)
                string_char         String?         @default("abcdefgh")
                string_text         String?         @default("abcdefgh")
                string_varchar      String?         @default("abcd")
                time_date           DateTime?       @default(dbgenerated())
                time_time           DateTime?       @default(now())
                time_timestamp      DateTime?       @default(now())
                time_timestamptz    DateTime?       @default(now())
            }
        "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]

async fn introspecting_a_default_value_as_dbgenerated_should_work(api: &TestApi) {
    let sequence = format!("CREATE SEQUENCE test_seq START 1");
    let color = format!("CREATE Type color as Enum ('black', 'white')");

    api.database().execute_raw(&sequence, &[]).await.unwrap();
    api.database().execute_raw(&color, &[]).await.unwrap();

    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("string_static_text text Default 'test'");
                t.inject_custom("string_static_text_null text Default Null");
                t.inject_custom("string_static_char char(5) Default 'test'");
                t.inject_custom("string_static_varchar varchar(5) Default 'test'");
                t.inject_custom("string_function text Default 'Concatenated'||E'\n'");
                t.inject_custom("int_static Integer DEFAULT 2");
                t.inject_custom("int_serial Serial4");
                t.inject_custom("int_function Integer DEFAULT EXTRACT(year from TIMESTAMP '2001-02-16 20:38:40')");
                t.inject_custom("int_sequence Integer DEFAULT nextval('test_seq')"); // todo this is not recognized as autoincrement
                t.inject_custom("float_static Float DEFAULT 1.43");
                t.inject_custom("boolean_static Boolean DEFAULT true");
                t.inject_custom("datetime_now_current TIMESTAMP DEFAULT CURRENT_TIMESTAMP");
                t.inject_custom("datetime_now TIMESTAMP DEFAULT NOW()");
                t.inject_custom("datetime_now_lc TIMESTAMP DEFAULT now()");
                t.inject_custom("enum_static color DEFAULT 'black'");
            });
        })
        .await;

    let dm = r#"
            model Test {
                boolean_static          Boolean?    @default(true)
                datetime_now            DateTime?   @default(now())
                datetime_now_current    DateTime?   @default(now())
                datetime_now_lc         DateTime?   @default(now())
                enum_static             color?      @default(black)
                float_static            Float?      @default(1.43)
                id                      Int         @default(autoincrement()) @id
                int_function            Int?        @default(dbgenerated())
                int_sequence            Int?        @default(dbgenerated())
                int_serial              Int        @default(autoincrement())
                int_static              Int?        @default(2)
                string_function         String?     @default(dbgenerated())
                string_static_char      String?     @default("test")
                string_static_text      String?     @default("test")
                string_static_text_null String?     
                string_static_varchar   String?     @default("test")
            }
            
           enum color{
                black
                white
           }
        "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_an_unsupported_type_should_comment_it_out(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("network_inet inet");
                t.inject_custom("network_mac  macaddr");
            });
        })
        .await;

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(
        &warnings,
        "[{\"code\":3,\"message\":\"These fields were commented out because we currently do not support their types.\",\"affected\":[{\"model\":\"Test\",\"field\":\"network_mac\",\"tpe\":\"macaddr\"}]}]"
    );

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, "model Test {\n  id             Int      @default(autoincrement()) @id\n  network_inet   String?\n  // This type is currently not supported.\n  // network_mac macaddr?\n}");
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_legacy_m_to_n_relation_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.inject_custom("id integer PRIMARY KEY");
            });
            migration.create_table("Category", |t| {
                t.inject_custom("id integer PRIMARY KEY");
            });

            migration.create_table("_CategoryToPost", |t| {
                t.inject_custom("A integer NOT NULL REFERENCES \"Category\"(id) ON DELETE CASCADE ON UPDATE CASCADE");
                t.inject_custom("B integer NOT NULL REFERENCES \"Post\"(id) ON DELETE CASCADE ON UPDATE CASCADE");
            });
        })
        .await;
    let unique = "CREATE UNIQUE INDEX _CategoryToPost_AB_unique ON \"_CategoryToPost\"(\"a\",\"b\" )";
    let index = "CREATE  INDEX _CategoryToPost_AB_index ON \"_CategoryToPost\"(\"b\" )";

    api.database().execute_raw(unique, &[]).await.unwrap();
    api.database().execute_raw(index, &[]).await.unwrap();

    let dm = r#"
            model Category {
              id            Int    @id
              Post          Post[]
            }

            model Post {
              id            Int    @id
              Category      Category[]
            }
        "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_default_values_on_lists_should_be_ignored(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("ints Integer[] DEFAULT array[]::Integer[]");
                t.inject_custom("ints2 Integer[] DEFAULT '{}'");
            });
        })
        .await;

    let dm = r#"
            datasource pg {
              provider = "postgres"
              url = "postgresql://localhost:5432"
            }

            model User {
               id      Int @id @default(autoincrement())
               ints    Int []
               ints2   Int []
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
