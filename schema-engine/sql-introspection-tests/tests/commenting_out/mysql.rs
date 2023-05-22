use barrel::types;
use sql_introspection_tests::{test_api::*, TestResult};

#[test_connector(tags(Mysql))]
async fn a_table_without_required_uniques(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("opt_unique", types::integer().unique(true).nullable(true));
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Post {
          id         Int
          opt_unique Int? @unique(map: "opt_unique")

          @@ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn a_table_without_uniques_should_ignore(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_idx", types::index(["user_id"]));

                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Post {
          id      Int
          user_id Int
          User    User @relation(fields: [user_id], references: [id], map: "user_id")

          @@index([user_id])
          @@ignore
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[] @ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn remapping_field_names_to_empty_mysql(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("1", types::text());
                t.add_column("last", types::integer().increments(true));

                t.add_constraint("User_pkey", types::primary_constraint(vec!["last"]));
            });
        })
        .await?;

    let dm = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          /// This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 String @map("1") @db.Text
          last Int @id @default(autoincrement())
        }
    "#]];

    api.expect_datamodel(&dm).await;

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn partition_table_gets_comment(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE `blocks` (
            id INT NOT NULL AUTO_INCREMENT,
            PRIMARY KEY (id)
        );

        ALTER TABLE blocks
        PARTITION BY HASH (id)
        PARTITIONS 2;
    "#,
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
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table is a partition table and requires additional setup for migrations. Visit https://pris.ly/d/partition-tables for more info.
        model blocks {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mysql8), exclude(Vitess))]
async fn mysql_multi_row_index_warning(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE customers (
            id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
            modified DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            custinfo JSON,
            INDEX zips( (CAST(custinfo->'$.zipcode' AS UNSIGNED ARRAY)) )
        );

        CREATE TABLE customers_2 (
            id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
            created DATETIME DEFAULT CURRENT_TIMESTAMP,
            modified DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            custinfo JSON,
            INDEX customers_2_created_idx (created, (CAST(custinfo->'$.zipcode' AS UNSIGNED ARRAY))),
            UNIQUE comp(modified, (CAST(custinfo->'$.zipcode' AS UNSIGNED ARRAY)))
        );

        CREATE TABLE my_table (
            id INT NOT NULL AUTO_INCREMENT,
            name VARCHAR(50) NOT NULL,
            email VARCHAR(50) NOT NULL,
            age INT NOT NULL,
            PRIMARY KEY (id),
            INDEX name_age (name, age),
            CONSTRAINT unique_name_email UNIQUE (name, email)
        );
    "#,
    )
    .await;

    let expected = expect![[r#"
        *** WARNING ***

        These tables contain multi-value indices, which are not yet fully supported. Read more: https://pris.ly/d/mysql-multi-row-index
          - "customers"
          - "customers_2"
    "#]];

    api.expect_warnings(&expected).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains multi-value indices, which are not yet fully supported. Visit https://pris.ly/d/mysql-multi-row-index for more info.
        model customers {
          id       BigInt    @id @default(autoincrement())
          modified DateTime? @default(now()) @db.DateTime(0)
          custinfo Json?
        }

        /// This table contains multi-value indices, which are not yet fully supported. Visit https://pris.ly/d/mysql-multi-row-index for more info.
        model customers_2 {
          id       BigInt    @id @default(autoincrement())
          created  DateTime? @default(now()) @db.DateTime(0)
          modified DateTime? @default(now()) @db.DateTime(0)
          custinfo Json?

          @@index([created])
        }

        model my_table {
          id    Int    @id @default(autoincrement())
          name  String @db.VarChar(50)
          email String @db.VarChar(50)
          age   Int

          @@unique([name, email], map: "unique_name_email")
          @@index([name, age], map: "name_age")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}
