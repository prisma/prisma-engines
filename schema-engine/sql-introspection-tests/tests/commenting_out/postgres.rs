use barrel::types;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn relations_between_ignored_models_should_not_have_field_level_ignores(api: &mut TestApi) -> TestResult {
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
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Post {
          id      Unsupported("macaddr") @id
          user_id Unsupported("macaddr")
          User    User                   @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
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
async fn fields_we_cannot_sanitize_are_commented_out_and_warned(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "Test" (
            "id" SERIAL PRIMARY KEY,
            "12" INT NOT NULL
        );
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        model Test {
          id Int @id @default(autoincrement())
          /// This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 12 Int @map("12")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute:
          - Model: "Test", field(s): ["12"]
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn unsupported_type_keeps_its_usages(api: &mut TestApi) -> TestResult {
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

    let expected = expect![[r#"
        *** WARNING ***

        These fields are not supported by Prisma Client, because Prisma currently does not support their types:
          - Model: "Test", field: "broken", original data type: "macaddr"
    "#]];

    api.expect_warnings(&expected).await;

    let dm = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        model Test {
          id     Int                    @unique
          dummy  Int
          broken Unsupported("macaddr")

          @@id([broken, dummy])
          @@unique([broken, dummy], map: "unique")
          @@index([broken, dummy], map: "non_unique")
        }
    "#]];

    api.expect_datamodel(&dm).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_with_only_an_unsupported_id(api: &mut TestApi) -> TestResult {
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

    let expected = expect![[r#"
        *** WARNING ***

        The following models were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client:
          - "Test"

        These fields are not supported by Prisma Client, because Prisma currently does not support their types:
          - Model: "Test", field: "network_mac", original data type: "macaddr"
    "#]];

    api.expect_warnings(&expected).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Test {
          dummy       Int
          network_mac Unsupported("macaddr") @id @default(dbgenerated("'08:00:2b:01:02:03'::macaddr"))

          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn a_table_with_unsupported_types_in_a_relation(api: &mut TestApi) -> TestResult {
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
async fn dbgenerated_in_unsupported(api: &mut TestApi) -> TestResult {
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
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
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
async fn commenting_out_a_table_without_columns(api: &mut TestApi) -> TestResult {
    api.raw_cmd("CREATE TABLE \"Test\" ();").await;

    let expected = expect![[r#"
        *** WARNING ***

        The following models were commented out as we could not retrieve columns for them. Please check your privileges:
          - "Test"
    "#]];

    api.expect_warnings(&expected).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        /// We could not retrieve columns for the underlying table. Either it has none or you are missing rights to see them. Please check your privileges.
        // model Test {
        // }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn ignore_on_back_relation_field_if_pointing_to_ignored_model(api: &mut TestApi) -> TestResult {
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
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
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
// partition tables without an workaround (see the following tests for details).
#[test_connector(
    tags(Postgres11, Postgres12, Postgres13, Postgres14, Postgres15, Postgres16),
    exclude(CockroachDb)
)]
async fn partition_table_gets_comment(api: &mut TestApi) -> TestResult {
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

    let expected = expect![[r#"
        *** WARNING ***

        These tables are partition tables, which are not yet fully supported:
          - "blocks"
    "#]];

    api.expect_warnings(&expected).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
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

// There is no way to make this work on Postgres10 currently. We can define UNIQUE or PK
// constraints on each of the parttions, but we are not allowed to define one on the main table.
// Our introspection currently only reads the propertieds/index properties for the main table, so
// these models will always be ignored.
#[test_connector(tags(Postgres), exclude(Postgres9, CockroachDb))]
async fn partition_table_gets_postgres10(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
CREATE TABLE IF NOT EXISTS blocks
(
    id int NOT NULL
) PARTITION BY RANGE (id);

CREATE TABLE blocks_p1_0 PARTITION OF blocks
    FOR VALUES FROM (0) TO (1000);

CREATE TABLE blocks_p2_0 PARTITION OF blocks
    FOR VALUES FROM (1001) TO (2000);

ALTER TABLE blocks_p1_0 ADD CONSTRAINT b1_unique UNIQUE (id);
ALTER TABLE blocks_p2_0 ADD CONSTRAINT b2_unique UNIQUE (id);
    "#,
    )
    .await;

    let expected = expect![[r#"
        *** WARNING ***

        The following models were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client:
          - "blocks"

        These tables are partition tables, which are not yet fully supported:
          - "blocks"
    "#]];

    api.expect_warnings(&expected).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        /// This table is a partition table and requires additional setup for migrations. Visit https://pris.ly/d/partition-tables for more info.
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model blocks {
          id Int

          @@ignore
        }
    "#]];
    api.expect_datamodel(&expected).await;
    Ok(())
}

// Postgres9 does not support row level security
#[test_connector(tags(Postgres), exclude(CockroachDb, Postgres9))]
async fn row_level_security_warning(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
-- Create a test table
CREATE TABLE foo (
    id SERIAL PRIMARY KEY,
    -- We use this row to security
    owner VARCHAR(30) NOT NULL
);

ALTER TABLE foo ENABLE ROW LEVEL SECURITY; "#,
    )
    .await;

    let expected = expect![[r#"
        *** WARNING ***

        These tables contain row level security, which is not yet fully supported. Read more: https://pris.ly/d/row-level-security
          - "foo"
    "#]];

    api.expect_warnings(&expected).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        /// This model contains row level security and requires additional setup for migrations. Visit https://pris.ly/d/row-level-security for more info.
        model foo {
          id    Int    @id @default(autoincrement())
          owner String @db.VarChar(30)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}
