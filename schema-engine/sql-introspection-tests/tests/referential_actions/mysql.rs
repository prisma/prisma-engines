use sql_introspection_tests::test_api::*;

// Older versions of MySQL (5.6+) raise a syntax error on `CREATE TABLE` declarations with `SET DEFAULT` referential actions,
// so we can skip introspecting those. MariaDb 10.0 suffers from the same issue.
// We should see validation warnings on MySQL 8+.
#[test_connector(tags(Mysql8), exclude(Vitess))]
async fn introspect_set_default_should_warn(api: &mut TestApi) -> TestResult {
    let setup = r#"
      CREATE TABLE `SomeUser` (
          `id` INTEGER NOT NULL AUTO_INCREMENT,
      
          PRIMARY KEY (`id`)
      ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
      
      CREATE TABLE `Post` (
          `id` INTEGER NOT NULL AUTO_INCREMENT,
          `userId` INTEGER NULL DEFAULT 3,
      
          PRIMARY KEY (`id`)
      ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
      
      ALTER TABLE `Post` ADD CONSTRAINT `Post_userId_fkey`
        FOREIGN KEY (`userId`) REFERENCES `SomeUser`(`id`)
        ON DELETE SET DEFAULT ON UPDATE SET DEFAULT;
    "#;

    api.raw_cmd(setup).await;

    let expected_schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Post {
          id       Int       @id @default(autoincrement())
          userId   Int?      @default(3)
          SomeUser SomeUser? @relation(fields: [userId], references: [id], onDelete: SetDefault, onUpdate: SetDefault)

          @@index([userId], map: "Post_userId_fkey")
        }

        model SomeUser {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected_schema.assert_eq(&api.introspect().await?);
    let schema = psl::parse_schema(expected_schema.data())?;

    let warning_messages = schema
        .diagnostics
        .warnings_to_pretty_string("schema.prisma", schema.db.source_assert_single());

    let expected_validation = expect![[r#"
        [1;93mwarning[0m: [1mMySQL does not actually support the `SetDefault` referential action, so using it may result in unexpected errors. Read more at https://pris.ly/d/mysql-set-default [0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m          userId   Int?      @default(3)
        [1;94m14 | [0m          SomeUser SomeUser? @relation(fields: [userId], references: [id], [1;93monDelete: SetDefault[0m, onUpdate: SetDefault)
        [1;94m   | [0m
        [1;93mwarning[0m: [1mMySQL does not actually support the `SetDefault` referential action, so using it may result in unexpected errors. Read more at https://pris.ly/d/mysql-set-default [0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m          userId   Int?      @default(3)
        [1;94m14 | [0m          SomeUser SomeUser? @relation(fields: [userId], references: [id], onDelete: SetDefault, [1;93monUpdate: SetDefault[0m)
        [1;94m   | [0m
    "#]];
    expected_validation.assert_eq(&warning_messages);

    Ok(())
}
