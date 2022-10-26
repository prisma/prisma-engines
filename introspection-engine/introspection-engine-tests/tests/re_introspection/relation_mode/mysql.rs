// @@map
mod at_at_map {
    use indoc::indoc;
    use introspection_engine_tests::test_api::*;

    // referentialIntegrity = "prisma" with @@map loses track of the relation policy ("prisma") and of @relations.
    #[test_connector(tags(Mysql), exclude(Vitess))]
    async fn referential_integrity_prisma_at_map_map(api: &TestApi) -> TestResult {
        let init = formatdoc! {r#"
            CREATE TABLE `foo_table` (
                `id` INTEGER NOT NULL,
                `bar_id` INTEGER NOT NULL,
            
                UNIQUE INDEX `foo_table_bar_id_key`(`bar_id`),
                PRIMARY KEY (`id`)
            );

            CREATE TABLE `bar_table` (
                `id` INTEGER NOT NULL,
            
                PRIMARY KEY (`id`)
            );
        "#};

        api.raw_cmd(&init).await;

        let input = indoc! {r#"
            generator client {
                provider        = "prisma-client-js"
                previewFeatures = ["referentialIntegrity"]
            }

            datasource db {
                provider             = "mysql"
                url                  = env("TEST_DATABASE_URL")
                referentialIntegrity = "prisma"
            }

            model Foo {
                id     Int @id
                bar    Bar @relation(fields: [bar_id], references: [id])
                bar_id Int @unique

                @@map("foo_table")
            }

            model Bar {
                id  Int  @id
                foo Foo?

                @@map("bar_table")
            }
        "#};

        let expected = expect![[r#"
            generator client {
              provider        = "prisma-client-js"
              previewFeatures = ["referentialIntegrity"]
            }

            datasource db {
              provider = "mysql"
              url      = env("TEST_DATABASE_URL")
            }

            model Foo {
              id     Int @id
              bar    Bar @relation(fields: [bar_id], references: [id])
              bar_id Int @unique

              @@map("foo_table")
            }

            model Bar {
              id  Int  @id
              foo Foo?

              @@map("bar_table")
            }
        "#]];

        let result = api.re_introspect_config(input).await?;
        expected.assert_eq(&result);

        Ok(())
    }

    // referentialIntegrity = "foreignKeys" with @@map loses track of the relation policy ("foreignKeys"), but preserves @relations, which are moved to the bottom.
    #[test_connector(tags(Mysql), exclude(Vitess))]
    async fn referential_integrity_foreign_keys_at_map_map(api: &TestApi) -> TestResult {
        let init = formatdoc! {r#"
            CREATE TABLE `foo_table` (
                `id` INTEGER NOT NULL,
                `bar_id` INTEGER NOT NULL,
            
                UNIQUE INDEX `foo_table_bar_id_key`(`bar_id`),
                PRIMARY KEY (`id`)
            );

            CREATE TABLE `bar_table` (
                `id` INTEGER NOT NULL,
            
                PRIMARY KEY (`id`)
            );

            ALTER TABLE `foo_table` ADD CONSTRAINT `foo_table_bar_id_fkey` FOREIGN KEY (`bar_id`) REFERENCES `bar_table`(`id`) ON DELETE RESTRICT ON UPDATE CASCADE;
        "#};

        api.raw_cmd(&init).await;

        let input = indoc! {r#"
            generator client {
                provider        = "prisma-client-js"
                previewFeatures = ["referentialIntegrity"]
            }

            datasource db {
                provider             = "mysql"
                url                  = env("TEST_DATABASE_URL")
                referentialIntegrity = "foreignKeys"
            }

            model Foo {
                id     Int @id
                bar    Bar @relation(fields: [bar_id], references: [id])
                bar_id Int @unique

                @@map("foo_table")
            }

            model Bar {
                id  Int  @id
                foo Foo?

                @@map("bar_table")
            }
        "#};

        let expected = expect![[r#"
            generator client {
              provider        = "prisma-client-js"
              previewFeatures = ["referentialIntegrity"]
            }

            datasource db {
              provider = "mysql"
              url      = env("TEST_DATABASE_URL")
            }

            model Foo {
              id     Int @id
              bar_id Int @unique
              bar    Bar @relation(fields: [bar_id], references: [id])

              @@map("foo_table")
            }

            model Bar {
              id  Int  @id
              foo Foo?

              @@map("bar_table")
            }
        "#]];

        let result = api.re_introspect_config(input).await?;
        expected.assert_eq(&result);

        Ok(())
    }

    // relationMode = "prisma" with @@map preserves the relation policy ("prisma"), but loses track of @relations.
    #[test_connector(tags(Mysql), exclude(Vitess))]
    async fn relation_mode_prisma_at_map_map(api: &TestApi) -> TestResult {
        let init = formatdoc! {r#"
            CREATE TABLE `foo_table` (
                `id` INTEGER NOT NULL,
                `bar_id` INTEGER NOT NULL,
            
                UNIQUE INDEX `foo_table_bar_id_key`(`bar_id`),
                PRIMARY KEY (`id`)
            );

            CREATE TABLE `bar_table` (
                `id` INTEGER NOT NULL,
            
                PRIMARY KEY (`id`)
            );
        "#};

        api.raw_cmd(&init).await;

        let input = indoc! {r#"
            generator client {
                provider        = "prisma-client-js"
                previewFeatures = ["referentialIntegrity"]
            }

            datasource db {
                provider     = "mysql"
                url          = env("TEST_DATABASE_URL")
                relationMode = "prisma"
            }

            model Foo {
                id     Int @id
                bar    Bar @relation(fields: [bar_id], references: [id])
                bar_id Int @unique

                @@map("foo_table")
            }

            model Bar {
                id  Int  @id
                foo Foo?

                @@map("bar_table")
            }
        "#};

        let expected = expect![[r#"
            generator client {
              provider        = "prisma-client-js"
              previewFeatures = ["referentialIntegrity"]
            }

            datasource db {
              provider     = "mysql"
              url          = env("TEST_DATABASE_URL")
              relationMode = "prisma"
            }

            model Foo {
              id     Int @id
              bar    Bar @relation(fields: [bar_id], references: [id])
              bar_id Int @unique

              @@map("foo_table")
            }

            model Bar {
              id  Int  @id
              foo Foo?

              @@map("bar_table")
            }
        "#]];

        let result = api.re_introspect_config(input).await?;
        expected.assert_eq(&result);

        Ok(())
    }

    // relationMode = "foreignKeys" with @@map preserves the relation policy ("foreignKeys") and @relations, which are moved to the bottom.
    #[test_connector(tags(Mysql), exclude(Vitess))]
    async fn relation_mode_foreign_keys_at_map_map(api: &TestApi) -> TestResult {
        let init = formatdoc! {r#"
            CREATE TABLE `foo_table` (
                `id` INTEGER NOT NULL,
                `bar_id` INTEGER NOT NULL,
            
                UNIQUE INDEX `foo_table_bar_id_key`(`bar_id`),
                PRIMARY KEY (`id`)
            );

            CREATE TABLE `bar_table` (
                `id` INTEGER NOT NULL,
            
                PRIMARY KEY (`id`)
            );

            ALTER TABLE `foo_table` ADD CONSTRAINT `foo_table_bar_id_fkey` FOREIGN KEY (`bar_id`) REFERENCES `bar_table`(`id`) ON DELETE RESTRICT ON UPDATE CASCADE;
        "#};

        api.raw_cmd(&init).await;

        let input = indoc! {r#"
            generator client {
                provider        = "prisma-client-js"
                previewFeatures = ["referentialIntegrity"]
            }

            datasource db {
                provider     = "mysql"
                url          = env("TEST_DATABASE_URL")
                relationMode = "foreignKeys"
            }

            model Foo {
                id     Int @id
                bar    Bar @relation(fields: [bar_id], references: [id])
                bar_id Int @unique

                @@map("foo_table")
            }

            model Bar {
                id  Int  @id
                foo Foo?

                @@map("bar_table")
            }
        "#};

        let expected = expect![[r#"
            generator client {
              provider        = "prisma-client-js"
              previewFeatures = ["referentialIntegrity"]
            }

            datasource db {
              provider     = "mysql"
              url          = env("TEST_DATABASE_URL")
              relationMode = "foreignKeys"
            }

            model Foo {
              id     Int @id
              bar_id Int @unique
              bar    Bar @relation(fields: [bar_id], references: [id])

              @@map("foo_table")
            }

            model Bar {
              id  Int  @id
              foo Foo?

              @@map("bar_table")
            }
        "#]];

        let result = api.re_introspect_config(input).await?;
        expected.assert_eq(&result);

        Ok(())
    }

    // @relations are moved to the bottom of the model even when no referentialIntegrity/relationMode is used and @@map is used.
    #[test_connector(tags(Mysql), exclude(Vitess))]
    async fn no_relation_at_map_map(api: &TestApi) -> TestResult {
        let init = formatdoc! {r#"
          CREATE TABLE `foo_table` (
              `id` INTEGER NOT NULL,
              `bar_id` INTEGER NOT NULL,
          
              UNIQUE INDEX `foo_table_bar_id_key`(`bar_id`),
              PRIMARY KEY (`id`)
          );

          CREATE TABLE `bar_table` (
              `id` INTEGER NOT NULL,
          
              PRIMARY KEY (`id`)
          );

          ALTER TABLE `foo_table` ADD CONSTRAINT `foo_table_bar_id_fkey` FOREIGN KEY (`bar_id`) REFERENCES `bar_table`(`id`) ON DELETE RESTRICT ON UPDATE CASCADE;
      "#};

        api.raw_cmd(&init).await;

        let input = indoc! {r#"
          datasource db {
              provider = "mysql"
              url      = env("TEST_DATABASE_URL")
          }

          model Foo {
              id     Int @id
              bar    Bar @relation(fields: [bar_id], references: [id])
              bar_id Int @unique

              @@map("foo_table")
          }

          model Bar {
              id  Int  @id
              foo Foo?

              @@map("bar_table")
          }
      "#};

        let expected = expect![[r#"
          datasource db {
            provider = "mysql"
            url      = env("TEST_DATABASE_URL")
          }

          model Foo {
            id     Int @id
            bar_id Int @unique
            bar    Bar @relation(fields: [bar_id], references: [id])

            @@map("foo_table")
          }

          model Bar {
            id  Int  @id
            foo Foo?

            @@map("bar_table")
          }
      "#]];

        let result = api.re_introspect_config(input).await?;
        expected.assert_eq(&result);

        Ok(())
    }
}
