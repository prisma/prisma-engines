use barrel::types;
use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
async fn negative_default_values_should_work(api: &TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "Blog" (
            id          SERIAL PRIMARY KEY,
            int         INT4 NOT NULL DEFAULT 1,
            neg_int     INT4 NOT NULL DEFAULT -1,
            float       FLOAT4 NOT NULL DEFAULT 2.1,
            neg_float   FLOAT4 NOT NULL DEFAULT -2.1,
            bigint      INT8 NOT NULL DEFAULT 3,
            neg_bigint  INT8 NOT NULL DEFAULT -3
        )
    "#;

    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Blog {
          id         BigInt @id @default(autoincrement())
          int        Int    @default(1)
          neg_int    Int    @default(-1)
          float      Float  @default(2.1) @db.Float4
          neg_float  Float  @default(-2.1) @db.Float4
          bigint     BigInt @default(3)
          neg_bigint BigInt @default(-3)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn should_ignore_prisma_helper_tables(api: &TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "Blog" (
            id SERIAL PRIMARY KEY
        );

        CREATE TABLE "_RelayId" (
            id SERIAL PRIMARY KEY,
            stablemodelidentifier STRING NOT NULL
        );

        CREATE TABLE "_Migration" (
            revision STRING NOT NULL,
            name STRING NOT NULL,
            datamodel STRING NOT NULL,
            status STRING NOT NULL,
            applied STRING NOT NULL,
            rolled_back STRING NOT NULL,
            datamodel_steps STRING NOT NULL,
            database_migration STRING NOT NULL,
            errors STRING NOT NULL,
            started_at STRING NOT NULL,
            finished_at STRING NOT NULL
        );

        CREATE TABLE "_prisma_migrations" (
            id SERIAL PRIMARY KEY,
            checksum STRING NOT NULL,
            finished_at STRING,
            migration_name STRING,
            logs STRING,
            rolled_back_at STRING,
            started_at STRING NOT NULL,
            applied_steps_count INT4
        );
    "#;

    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Blog {
          id BigInt @id @default(autoincrement())
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn a_table_with_partial_indexes_should_ignore_them(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("pages", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("staticId", types::integer().nullable(false));
                t.add_column("latest", types::integer().nullable(false));
                t.add_column("other", types::integer().nullable(false));
                t.add_index("full", types::index(vec!["other"]).unique(true));
                t.add_partial_index("partial", types::index(vec!["staticId"]).unique(true), "latest = 1");

                t.add_constraint("pages_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        model pages {
          id       BigInt @id @default(autoincrement())
          staticId Int
          latest   Int
          other    Int    @unique(map: "full")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn duplicate_fks_should_ignore_one_of_them(api: &TestApi) -> TestResult {
    use barrel::types;

    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer().nullable(true));

                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
            });

            migration.change_table("Post", |t| {
                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            })
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      BigInt @id @default(autoincrement())
          user_id Int?
          User    User?  @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   BigInt @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
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

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          id    BigInt @id @default(autoincrement())
          ints  Int[]
          ints2 Int[]
        }
    "#]];
    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn default_values(api: &TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "Test" (
            id SERIAL PRIMARY KEY,
            string_static_char CHAR(5) DEFAULT 'test',
            string_static_char_null CHAR(5) DEFAULT NULL,
            string_static_varchar VARCHAR(5) DEFAULT 'test',
            int_static INT4 DEFAULT 2,
            float_static FLOAT4 DEFAULT 1.43,
            boolean_static BOOL DEFAULT true,
            datetime_now TIMESTAMPTZ DEFAULT current_timestamp()
        );
    "#;

    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Test {
          id                      BigInt    @id @default(autoincrement())
          string_static_char      String?   @default("test") @db.Char(5)
          string_static_char_null String?   @db.Char(5)
          string_static_varchar   String?   @default("test") @db.String(5)
          int_static              Int?      @default(2)
          float_static            Float?    @default(1.43) @db.Float4
          boolean_static          Boolean?  @default(true)
          datetime_now            DateTime? @default(now()) @db.Timestamptz(6)
        }
    "#]];
    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn a_simple_table_with_gql_types(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("bool", types::boolean());
                t.add_column("float", types::float());
                t.add_column("date", types::datetime());
                t.add_column("id", types::integer().increments(true));
                t.add_column("int", types::integer());
                t.add_column("string", types::text());

                t.add_constraint("Blog_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Blog {
          bool   Boolean
          float  Float
          date   DateTime @db.Timestamp(6)
          id     BigInt   @id @default(autoincrement())
          int    Int
          string String
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn introspecting_a_table_with_json_type_must_work_cockroach(api: &TestApi) -> TestResult {
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
            id      BigInt @id @default(autoincrement())
            json    Json
        }
    "#};

    let result = api.introspect().await?;

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

// Cockroach can return non-deterministic results if the UNIQUE constraint is defined twice
// (it does not collapse similar unique constraints). This variation does not include the
// doubly defined unique constraint.
#[test_connector(tags(CockroachDb))]
async fn a_table_with_non_id_autoincrement_cockroach(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::integer());
                t.add_column("authorId", types::serial().unique(true));

                t.add_constraint("Test_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Test {
            id       Int @id
            authorId BigInt @default(autoincrement()) @unique
        }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn introspecting_json_defaults_on_cockroach(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
       CREATE TABLE "A" (
           id INTEGER NOT NULL Primary Key,
           json Json Default '[]'::json,
           jsonb JsonB Default '{}'::jsonb
         );

       "#};
    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model A {
          id    Int   @id
          json  Json? @default("[]")
          jsonb Json? @default("{}")
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
