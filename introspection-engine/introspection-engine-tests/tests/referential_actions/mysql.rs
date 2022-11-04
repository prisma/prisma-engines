use introspection_engine_tests::test_api::*;

#[test_connector(tags(Mysql))]
async fn introspect_set_default_should_fail(api: &TestApi) -> TestResult {
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
    let validation = psl::parse_schema(expected_schema.data());

    let expected_validation = expect![[r#"
        [1;91merror[0m: [1mError validating: Invalid referential action: `SetDefault`. Allowed values: (`Cascade`, `Restrict`, `NoAction`, `SetNull`). `SetDefault` is invalid for MySQL when using `relationMode = \"foreignKeys\"`, as MySQL does not support the `SET DEFAULT` referential action.
        Learn more at https://github.com/prisma/prisma/issues/11498
        [0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m          userId   Int?      @default(3)
        [1;94m14 | [0m          SomeUser SomeUser? @relation(fields: [userId], references: [id], [1;91monDelete: SetDefault[0m, onUpdate: SetDefault)
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Invalid referential action: `SetDefault`. Allowed values: (`Cascade`, `Restrict`, `NoAction`, `SetNull`). `SetDefault` is invalid for MySQL when using `relationMode = \"foreignKeys\"`, as MySQL does not support the `SET DEFAULT` referential action.
        Learn more at https://github.com/prisma/prisma/issues/11498
        [0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m          userId   Int?      @default(3)
        [1;94m14 | [0m          SomeUser SomeUser? @relation(fields: [userId], references: [id], onDelete: SetDefault, [1;91monUpdate: SetDefault[0m)
        [1;94m   | [0m
    "#]];
    expected_validation.assert_eq(&validation.err().unwrap());

    Ok(())
}
