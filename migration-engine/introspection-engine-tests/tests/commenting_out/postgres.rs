use barrel::types;
use introspection_engine_tests::{assert_eq_json, test_api::*};
use serde_json::json;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn relations_between_ignored_models_should_not_have_field_level_ignores(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.inject_custom("id macaddr primary key not null");
            });
            migration.create_table("Post", |t| {
                t.inject_custom("id macaddr primary key not null");
                t.inject_custom("user_id macaddr not null");
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id      Unsupported("macaddr") @id
          user_id Unsupported("macaddr")
          User    User                   @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model User {
          id   Unsupported("macaddr") @id
          Post Post[]

          @@ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn fields_we_cannot_sanitize_are_commented_out_and_warned(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "Test" (
            "id" SERIAL PRIMARY KEY,
            "12" INT NOT NULL
        );
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Test {
          id Int @id @default(autoincrement())
          /// This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 12 Int @map("12")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        [
          {
            "code": 2,
            "message": "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute.",
            "affected": [
              {
                "model": "Test",
                "field": "12"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn unsupported_type_keeps_its_usages(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::integer().unique(true));
                t.add_column("dummy", types::integer());
                t.add_column("broken", types::custom("macaddr"));
                t.add_index("unique", types::index(vec!["broken", "dummy"]).unique(true));
                t.add_index("non_unique", types::index(vec!["broken", "dummy"]).unique(false));
                t.set_primary_key(&["broken", "dummy"]);
            });
        })
        .await?;

    let expected = json!([{
        "code": 3,
        "message": "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.",
        "affected": [
            {
                "model": "Test",
                "field": "broken",
                "tpe": "macaddr"
            }
        ]
    }]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let dm = expect![[r#"
        model Test {
          id     Int                    @unique
          dummy  Int
          broken Unsupported("macaddr")

          @@id([broken, dummy])
          @@unique([broken, dummy], map: "unique")
          @@index([broken, dummy], map: "non_unique")
        }
    "#]];

    let result = api.introspect_dml().await?;

    dm.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_with_only_an_unsupported_id(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("dummy", types::integer());
                t.add_column(
                    "network_mac",
                    types::custom("macaddr").primary(true).default("08:00:2b:01:02:03"),
                );
            });
        })
        .await?;

    let expected = json!([
        {
            "code": 1,
            "message": "The following models were ignored as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client.",
            "affected": [{
                "model": "Test"
            }]
        },
        {
            "code": 3,
            "message": "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.",
            "affected": [{
                "model": "Test",
                "field": "network_mac",
                "tpe": "macaddr"
            }]
        }
    ]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let dm = indoc! {r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Test {
          dummy       Int
          network_mac Unsupported("macaddr") @id @default(dbgenerated("'08:00:2b:01:02:03'::macaddr"))

          @@ignore
        }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_with_unsupported_types_in_a_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("ip cidr not null unique");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_ip cidr not null");
                t.add_foreign_key(&["user_ip"], "User", &["ip"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int                 @id @default(autoincrement())
          user_ip Unsupported("cidr")
          User    User                @relation(fields: [user_ip], references: [ip], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int                 @id @default(autoincrement())
          ip   Unsupported("cidr") @unique
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn dbgenerated_in_unsupported(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "Blog" (
          id SERIAL PRIMARY KEY,
          number INT DEFAULT 1,
          bigger_number INT DEFAULT sqrt(4),
          point POINT DEFAULT Point(0, 0)
        )
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Blog {
          id            Int                   @id @default(autoincrement())
          number        Int?                  @default(1)
          bigger_number Int?                  @default(dbgenerated("sqrt((4)::double precision)"))
          point         Unsupported("point")? @default(dbgenerated("point((0)::double precision, (0)::double precision)"))
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn commenting_out_a_table_without_columns(api: &TestApi) -> TestResult {
    api.raw_cmd("CREATE TABLE \"Test\" ();").await;

    let expected = json!([{
        "code": 14,
        "message": "The following models were commented out as we could not retrieve columns for them. Please check your privileges.",
        "affected": [
            {
                "model": "Test"
            }
        ]
    }]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// We could not retrieve columns for the underlying table. Either it has none or you are missing rights to see them. Please check your privileges.
        // model Test {
        // }
    "#]];
    api.expect_datamodel(&expected).await;
    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn ignore_on_back_relation_field_if_pointing_to_ignored_model(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("ip integer not null unique");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.inject_custom("user_ip integer not null ");
                t.add_foreign_key(&["user_ip"], "User", &["ip"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id      Int
          user_ip Int
          User    User @relation(fields: [user_ip], references: [ip], onDelete: NoAction, onUpdate: NoAction)

          @@ignore
        }

        model User {
          id   Int    @id @default(autoincrement())
          ip   Int    @unique
          Post Post[] @ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

// Postgres9 does not support partition tables, and Postgres10 does not support primary keys on
// partition tables.
#[test_connector(
    tags(Postgres11, Postgres12, Postgres13, Postgres14, Postgres15),
    exclude(CockroachDb)
)]
async fn partition_table_gets_comment(api: &TestApi) -> TestResult {
    api.raw_cmd(
        r#"
CREATE TABLE IF NOT EXISTS blocks
(
    id int NOT NULL,
    account text COLLATE pg_catalog."default" NOT NULL,
    block_source_id int,
    CONSTRAINT blocks_pkey PRIMARY KEY (account, id)
) PARTITION BY RANGE (id);


CREATE TABLE blocks_p1_0 PARTITION OF blocks
    FOR VALUES FROM (0) TO (1000);

CREATE TABLE blocks_p2_0 PARTITION OF blocks
    FOR VALUES FROM (1001) TO (2000);

ALTER TABLE blocks
      ADD CONSTRAINT block_source_block_fk FOREIGN KEY (block_source_id, account)
        REFERENCES blocks (id, account) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE CASCADE; "#,
    )
    .await;

    let expected = json!([{
        "code": 27,
        "message": "These tables are partition tables, which are not yet fully supported.",
        "affected": [
            {
                "model": "blocks"
            }
        ]
    }]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table is a partition table and requires additional setup for migrations. Visit https://pris.ly/d/partition-tables for more info.
        model blocks {
          id              Int
          account         String
          block_source_id Int?
          blocks          blocks?  @relation("blocksToblocks", fields: [block_source_id, account], references: [id, account], onDelete: Cascade, onUpdate: NoAction, map: "block_source_block_fk")
          other_blocks    blocks[] @relation("blocksToblocks")

          @@id([account, id])
        }
    "#]];
    api.expect_datamodel(&expected).await;
    Ok(())
}
