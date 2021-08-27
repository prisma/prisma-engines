use barrel::{functions, types};
use expect_test::expect;
use indoc::formatdoc;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Mysql57))]
async fn nul_default_bytes(api: &TestApi) -> TestResult {
    let create_table = indoc! {r#"
        CREATE TABLE nul_default_bytes
        (
            id  INT                  NOT NULL
                PRIMARY KEY,
            val BINARY(5) DEFAULT '\0\0\0\0\0' NOT NULL
        )
    "#};

    api.database().raw_cmd(create_table).await?;

    let expected = expect![[r#"
        model nul_default_bytes {
          id  Int   @id
          val Bytes @default(dbgenerated()) @db.Binary(5)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector]
async fn a_simple_table_with_gql_types(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("bool", types::boolean());
                t.add_column("float", types::float());
                t.add_column("date", types::datetime());
                t.add_column("id", types::primary());
                t.add_column("int", types::integer());
                t.add_column("string", types::text());
            });
        })
        .await?;

    let float_native = if api.sql_family().is_mssql() {
        "@db.Real"
    } else if api.sql_family().is_mysql() {
        "@db.Float"
    } else {
        ""
    };
    let timestamp_native = if api.sql_family().is_postgres() {
        "@db.Timestamp(6)"
    } else if api.sql_family().is_mysql() {
        "@db.DateTime(0)"
    } else {
        ""
    };

    let text_native = if api.sql_family().is_mssql() || api.sql_family().is_mysql() {
        "@db.Text"
    } else {
        ""
    };

    let dm = formatdoc! {r##"
        model Blog {{
            bool    Boolean
            float   Float {float_native}
            date    DateTime {timestamp_native}
            id      Int @id @default(autoincrement())
            int     Int
            string  String {text_native}
        }}
    "##, float_native = float_native, timestamp_native = timestamp_native, text_native = text_native};

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn should_ignore_prisma_helper_tables(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("_RelayId", move |t| {
                t.add_column("id", types::primary());
                t.add_column("stablemodelidentifier", types::text());
            });

            migration.create_table("_Migration", move |t| {
                t.add_column("revision", types::text());
                t.add_column("name", types::text());
                t.add_column("datamodel", types::text());
                t.add_column("status", types::text());
                t.add_column("applied", types::text());
                t.add_column("rolled_back", types::text());
                t.add_column("datamodel_steps", types::text());
                t.add_column("database_migration", types::text());
                t.add_column("errors", types::text());
                t.add_column("started_at", types::text());
                t.add_column("finished_at", types::text());
            });

            migration.create_table("_prisma_migrations", move |t| {
                t.add_column("id", types::primary());
                t.add_column("checksum", types::text());
                t.add_column("finished_at", types::text());
                t.add_column("migration_name", types::text());
                t.add_column("logs", types::text());
                t.add_column("rolled_back_at", types::text());
                t.add_column("started_at", types::text());
                t.add_column("applied_steps_count", types::text());
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Blog {
            id      Int @id @default(autoincrement())
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_table_with_compound_primary_keys(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::integer());
                t.add_column("authorId", types::integer());
                t.set_primary_key(&["id", "authorId"]);
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Blog {
            id Int
            authorId Int
            @@id([id, authorId])
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_table_with_unique_index(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("authorId", types::integer());
                t.add_index("test", types::index(vec!["authorId"]).unique(true));
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Blog {
            id       Int @id @default(autoincrement())
            authorId Int @unique
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_table_with_multi_column_unique_index(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("firstname", types::integer());
                t.add_column("lastname", types::integer());
                t.add_index("test", types::index(vec!["firstname", "lastname"]).unique(true));
            });
        })
        .await?;

    let dm = indoc! {r##"
        model User {
            id      Int @id @default(autoincrement())
            firstname Int
            lastname Int
            @@unique([firstname, lastname], name: "test")
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_table_with_required_and_optional_columns(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("requiredname", types::integer().nullable(false));
                t.add_column("optionalname", types::integer().nullable(true));
            });
        })
        .await?;

    let dm = indoc! {r##"
        model User {
            id      Int @id @default(autoincrement())
            requiredname Int
            optionalname Int?
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_table_with_default_values(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("bool", types::boolean().default(false));
                t.add_column("bool2", types::boolean().default(true));
                t.add_column("float", types::float().default(5.3));
                t.add_column("int", types::integer().default(5));
                t.add_column("string", types::char(4).default("Test"));
            });
        })
        .await?;

    let native_string = if !api.sql_family().is_sqlite() {
        "@db.Char(4)"
    } else {
        ""
    };
    let float_string = if api.sql_family().is_mysql() {
        "@db.Float"
    } else if api.sql_family().is_mssql() {
        "@db.Real"
    } else {
        ""
    };

    let dm = formatdoc! {r##"
        model User {{
            id     Int     @id @default(autoincrement())
            bool   Boolean @default(false)
            bool2  Boolean @default(true)
            float  Float   @default(5.3) {}
            int    Int     @default(5)
            string String  @default("Test") {}
        }}
    "##, float_string, native_string};

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_table_with_a_non_unique_index(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::integer());
                t.add_column("id", types::primary());
                t.add_index("test", types::index(vec!["a"]));
            });
        })
        .await?;

    let dm = indoc! {r##"
        model User {
            a       Int
            id      Int @id @default(autoincrement())
            @@index([a], name: "test")
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn a_table_with_a_multi_column_non_unique_index(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::integer());
                t.add_column("b", types::integer());
                t.add_column("id", types::primary());
                t.add_index("test", types::index(vec!["a", "b"]));
            });
        })
        .await?;

    let dm = indoc! { r##"
        model User {
            a  Int
            b  Int
            id Int @id @default(autoincrement())
            @@index([a,b], name: "test")
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

// SQLite does not have a serial type that's not a primary key.
#[test_connector(exclude(Sqlite))]
async fn a_table_with_non_id_autoincrement(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::integer().primary(true));
                t.add_column("authorId", types::serial().unique(true));
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Test {
            id       Int @id
            authorId Int @default(autoincrement()) @unique
        }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector]
async fn default_values(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", move |t| {
                t.add_column("id", types::primary());
                t.add_column(
                    "string_static_char",
                    types::custom("char(5)").default("test").nullable(true),
                );
                t.add_column(
                    "string_static_char_null",
                    types::r#char(5).default(types::null()).nullable(true),
                );
                t.add_column(
                    "string_static_varchar",
                    types::varchar(5).default("test").nullable(true),
                );
                t.add_column("int_static", types::integer().default(2).nullable(true));
                t.add_column("float_static", types::float().default(1.43).nullable(true));
                t.add_column("boolean_static", types::boolean().default(true).nullable(true));
                t.add_column(
                    "datetime_now",
                    types::datetime().default(functions::current_timestamp()).nullable(true),
                );
            });
        })
        .await?;

    let char_native = if !api.sql_family().is_sqlite() {
        "@db.Char(5)"
    } else {
        ""
    };
    let varchar_native = if !api.sql_family().is_sqlite() {
        "@db.VarChar(5)"
    } else {
        ""
    };

    let float_native = if api.sql_family().is_mssql() {
        "@db.Real"
    } else if api.sql_family().is_mysql() {
        "@db.Float"
    } else {
        ""
    };
    let timestamp_native = if api.sql_family().is_postgres() {
        "@db.Timestamp(6)"
    } else if api.sql_family().is_mysql() {
        "@db.DateTime(0)"
    } else {
        ""
    };

    let dm = formatdoc! { r#"
        model Test {{
            id                      Int       @id @default(autoincrement())
            string_static_char      String?   @default("test") {}
            string_static_char_null String? {}
            string_static_varchar   String?   @default("test") {}
            int_static              Int?      @default(2)
            float_static            Float?    @default(1.43) {}
            boolean_static          Boolean?  @default(true)
            datetime_now            DateTime? @default(now()) {}
        }}
    "#, char_native, char_native, varchar_native, float_native,  timestamp_native};

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Cockroach))]
async fn pg_default_value_as_dbgenerated(api: &TestApi) -> TestResult {
    let sequence = "CREATE SEQUENCE test_seq START 1".to_string();
    api.database().execute_raw(&sequence, &[]).await?;

    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("string_function text Default E'  ' || '>' || ' '");
                t.inject_custom("int_serial Serial4");
                t.inject_custom("int_function Integer DEFAULT EXTRACT(year from TIMESTAMP '2001-02-16 20:38:40')");
                t.inject_custom("int_sequence Integer DEFAULT nextval('test_seq')");
                t.inject_custom("datetime_now TIMESTAMP DEFAULT NOW()");
                t.inject_custom("datetime_now_lc TIMESTAMP DEFAULT now()");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Test {
          id              Int       @id @default(autoincrement())
          string_function String?   @default(dbgenerated("(('  '::text || '>'::text) || ' '::text)"))
          int_serial      Int       @default(autoincrement())
          int_function    Int?      @default(dbgenerated("date_part('year'::text, '2001-02-16 20:38:40'::timestamp without time zone)"))
          int_sequence    Int?      @default(autoincrement())
          datetime_now    DateTime? @default(now()) @db.Timestamp(6)
          datetime_now_lc DateTime? @default(now()) @db.Timestamp(6)
          }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

//todo maybe need to split due to
// no function default values on mysql 5.7 and 8.0 -.-
// maria db allows this
#[test_connector(tags(Mysql))]
async fn my_default_value_as_dbgenerated(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("datetime_now TIMESTAMP NULL DEFAULT CURRENT_TIMESTAMP");
                t.inject_custom("datetime_now_lc TIMESTAMP NULL DEFAULT current_timestamp");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Test {
            id                      Int                 @id @default(autoincrement())
            datetime_now            DateTime?           @default(now()) @db.Timestamp(0)
            datetime_now_lc         DateTime?           @default(now()) @db.Timestamp(0)
        }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn a_table_with_an_index_that_contains_expressions_should_be_ignored(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::integer().primary(true));
                t.add_column("parentId", types::integer().nullable(true));
                t.add_column("name", types::varchar(45).nullable(true));
                t.inject_custom("UNIQUE KEY `SampleTableUniqueIndexName` (`name`,(ifnull(`parentId`,-(1))))");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Test {
            id       Int     @id
            parentId Int?
            name     String? @db.VarChar(45)
        }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn default_values_on_lists_should_be_ignored(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("ints integer[] DEFAULT array[]::integer[]");
                t.inject_custom("ints2 integer[] DEFAULT '{}'");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model User {
            id      Int @id @default(autoincrement())
            ints    Int[]
            ints2   Int[]
        }
    "#};

    let result = api.introspect().await?;

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

// MySQL doesn't have partial indices.
#[test_connector(exclude(Mysql))]
async fn a_table_with_partial_indexes_should_ignore_them(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("pages", move |t| {
                t.add_column("id", types::primary());
                t.add_column("staticId", types::integer().nullable(false));
                t.add_column("latest", types::integer().nullable(false));
                t.add_column("other", types::integer().nullable(false));
                t.add_index("full", types::index(vec!["other"]).unique(true));
                t.add_partial_index("partial", types::index(vec!["staticId"]).unique(true), "latest = 1");
            });
        })
        .await?;

    let dm = indoc! {
        r#"
        model pages {
            id       Int     @id @default(autoincrement())
            staticId Int
            latest   Int
            other    Int     @unique
        }
        "#
    };

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn introspecting_a_table_with_json_type_must_work(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("json", types::json());
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Blog {
            id      Int @id @default(autoincrement())
            json    Json @db.Json
        }
    "#};

    let result = api.introspect().await?;

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(Mariadb))]
async fn different_default_values_should_work(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
                t.inject_custom("text Text Default \"one\"");
                t.inject_custom("`tinytext_string` tinytext COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT \"twelve\"");
                t.inject_custom("`tinytext_number_string` tinytext COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT \"1\"");
                t.inject_custom("`tinytext_number` tinytext COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 10");
                t.inject_custom("`tinytext_float` tinytext COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 1.0");
                t.inject_custom("`tinytext_short` tinytext COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 1");
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Blog {
          id                     Int     @id @default(autoincrement())
          text                   String? @default("one") @db.Text
          tinytext_string        String  @default("twelve") @db.TinyText
          tinytext_number_string String  @default("1") @db.TinyText
          tinytext_number        String  @default(dbgenerated("(10)")) @db.TinyText
          tinytext_float         String  @default(dbgenerated("(1.0)")) @db.TinyText
          tinytext_short         String  @default(dbgenerated("(1)")) @db.TinyText
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite))]
async fn negative_default_values_should_work(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
                t.add_column("int", types::integer().default(1));
                t.add_column("neg_int", types::integer().default(-1));
                t.add_column("float", types::float().default(2.1));
                t.add_column("neg_float", types::float().default(-2.1));
                t.add_column("big_int", types::custom("bigint").default(3));
                t.add_column("neg_big_int", types::custom("bigint").default(-3));
            });
        })
        .await?;

    let float_native = if api.sql_family().is_mysql() {
        "@db.Float"
    } else if api.sql_family().is_mssql() {
        "@db.Real"
    } else {
        ""
    };

    let dm = formatdoc! {r##"
        model Blog {{
          id                     Int         @id @default(autoincrement())
          int                    Int         @default(1)
          neg_int                Int         @default(-1)
          float                  Float       @default(2.1) {float_native}
          neg_float              Float       @default(-2.1) {float_native}
          big_int                BigInt      @default(3)
          neg_big_int            BigInt      @default(-3)
        }}
    "##, float_native = float_native};

    api.assert_eq_datamodels(&dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn partial_indexes_should_be_ignored_on_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
                t.add_column("int_col", types::integer());
                t.inject_custom("blob_col mediumblob");
                t.inject_custom("Index `partial_blob_col_index` (blob_col(10))");
                t.inject_custom("Index `partial_compound` (blob_col(10), int_col)");
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Blog {
          id                Int     @id @default(autoincrement())
          int_col           Int
          blob_col          Bytes?  @db.MediumBlob
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm, result);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn expression_indexes_should_be_ignored_on_sqlite(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
                t.add_column("author", types::text());
            });
            migration.inject_custom("CREATE INDEX author_lowercase_index ON Blog(LOWER(author));")
        })
        .await?;

    let dm = indoc! {r##"
        model Blog {
          id                Int     @id @default(autoincrement())
          author            String
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm, result);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn casing_should_not_lead_to_mix_ups(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("address", move |t| {
                t.inject_custom("addressid INT NOT NULL");
                t.inject_custom("PRIMARY KEY(addressid)");
            });

            migration.create_table("ADDRESS", move |t| {
                t.inject_custom("ADDRESSID INT NOT NULL");
                t.inject_custom("PRIMARY KEY(ADDRESSID)");
            });
            migration.create_table("Address", move |t| {
                t.inject_custom("AddressID INT NOT NULL AUTO_INCREMENT");
                t.inject_custom("PRIMARY KEY(AddressID)");
            });
        })
        .await?;

    let dm = indoc! {r##"
        model ADDRESS {
          ADDRESSID Int @id
        }

        model Address {
          AddressID Int @id @default(autoincrement())
        }

        model address {
          addressid Int @id
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm, result);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Mariadb))]
async fn unique_and_index_on_same_field_works_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.inject_custom(
                "create table users (
                       id serial primary key not null
                     );",
            )
        })
        .await?;

    let dm = indoc! {r##"
        model users {
          id BigInt @id @unique @default(autoincrement()) @db.UnsignedBigInt
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm, result);

    Ok(())
}

#[test_connector(tags(Mariadb))]
async fn unique_and_index_on_same_field_works_mariadb(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.inject_custom(
                "create table users (
                       id Integer primary key not null,
                       CONSTRAINT really_must_be_different UNIQUE (id)
                     );",
            )
        })
        .await?;

    let dm = indoc! {r##"
        model users {
          id Int @id @unique
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm, result);
    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn unique_and_index_on_same_field_works_sqlite(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.inject_custom(
                "create table users (
                       id Integer primary key not null unique
                     );",
            )
        })
        .await?;

    let dm = indoc! {r##"
        model users {
          id Int @id @unique @default(autoincrement())
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm, result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn unique_and_id_on_same_field_works_mssql(api: &TestApi) -> TestResult {
    let schema = api.schema_name();

    api.barrel()
        .execute(|migration| {
            migration.inject_custom(format!(
                "create table {}.users (
                       id int identity primary key,
                       constraint unique_and_index_same unique(id) 
                     );",
                schema
            ))
        })
        .await?;

    let dm = indoc! {r##"
        model users {
          id Int @id @unique @default(autoincrement())
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm, result);

    Ok(())
}

#[test_connector(tags(Postgres))]
// If multiple constraints are created in the create table statement Postgres seems to collapse them
// into the first named one. So on the db level there will be one named really_must_be_different that
// is both unique and primary. We only render it as @id then.
// If a later alter table statement adds another unique constraint then it is persisted as its own
// entity and can be introspected.
async fn unique_and_index_on_same_field_works_postgres(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.inject_custom(
                "create table users (
                       id Integer primary key not null,
                       CONSTRAINT really_must_be_different UNIQUE (id),
                       CONSTRAINT must_be_different UNIQUE (id)
                     );",
            )
        })
        .await?;

    let dm = indoc! {r##"
        model users {
          id Int @id
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm, result);

    api.barrel()
        .execute(|migration| migration.inject_custom("Alter table users Add Constraint z_unique Unique(id);"))
        .await?;

    let dm2 = indoc! {r##"
        model users {
          id Int @id @unique
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm2, result);

    Ok(())
}
