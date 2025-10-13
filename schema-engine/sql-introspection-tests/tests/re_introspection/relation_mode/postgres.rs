use indoc::indoc;
use sql_introspection_tests::test_api::*;

// referentialIntegrity="prisma" is renamed as relationMode="prisma", and @relations are preserved.
#[test_connector(tags(Postgres))]
async fn referential_integrity_prisma(api: &mut TestApi) -> TestResult {
    let init = indoc! {r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            "bar_id" INTEGER NOT NULL,

            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );

        CREATE TABLE "Bar" (
            "id" INTEGER NOT NULL,

            CONSTRAINT "Bar_pkey" PRIMARY KEY ("id")
        );

        CREATE UNIQUE INDEX "Foo_bar_id_key" ON "Foo"("bar_id");
    "#};

    api.raw_cmd(init).await;

    let input = indoc! {r#"
        generator client {
            provider = "prisma-client"
        }

        datasource db {
            provider             = "postgres"
            url                  = "dummy-url"
            referentialIntegrity = "prisma"
        }

        model Foo {
            id     Int @id
            bar    Bar @relation(fields: [bar_id], references: [id])
            bar_id Int @unique
        }

        model Bar {
            id  Int  @id
            foo Foo?
        }
    "#};

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider     = "postgres"
          url          = "dummy-url"
          relationMode = "prisma"
        }

        model Foo {
          id     Int @id
          bar_id Int @unique
          bar    Bar @relation(fields: [bar_id], references: [id])
        }

        model Bar {
          id  Int  @id
          foo Foo?
        }
    "#]];

    let result = api.re_introspect_config(input).await?;
    expected.assert_eq(&result);

    Ok(())
}

// referentialIntegrity="foreignKeys" is renamed as relationMode="foreignKeys", and @relations are preserved but moved to the bottom.
#[test_connector(tags(Postgres))]
async fn referential_integrity_foreign_keys(api: &mut TestApi) -> TestResult {
    let init = indoc! {r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            "bar_id" INTEGER NOT NULL,
        
            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
        
        CREATE TABLE "Bar" (
            "id" INTEGER NOT NULL,
        
            CONSTRAINT "Bar_pkey" PRIMARY KEY ("id")
        );
        
        CREATE UNIQUE INDEX "Foo_bar_id_key" ON "Foo"("bar_id");

        ALTER TABLE "Foo" ADD CONSTRAINT "Foo_bar_id_fkey" FOREIGN KEY ("bar_id") REFERENCES "Bar"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
    "#};

    api.raw_cmd(init).await;

    let input = indoc! {r#"
        generator client {
            provider = "prisma-client"
        }

        datasource db {
            provider             = "postgres"
            url                  = "dummy-url"
            referentialIntegrity = "foreignKeys"
        }

        model Foo {
            id     Int @id
            bar    Bar @relation(fields: [bar_id], references: [id])
            bar_id Int @unique
        }

        model Bar {
            id  Int  @id
            foo Foo?
        }
    "#};

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider     = "postgres"
          url          = "dummy-url"
          relationMode = "foreignKeys"
        }

        model Foo {
          id     Int @id
          bar_id Int @unique
          bar    Bar @relation(fields: [bar_id], references: [id])
        }

        model Bar {
          id  Int  @id
          foo Foo?
        }
    "#]];

    let result = api.re_introspect_config(input).await?;
    expected.assert_eq(&result);

    Ok(())
}

// relationMode="prisma" preserves the relation policy ("prisma") as well as @relations.
#[test_connector(tags(Postgres))]
async fn relation_mode_prisma(api: &mut TestApi) -> TestResult {
    let init = indoc! {r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            "bar_id" INTEGER NOT NULL,
        
            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
        
        CREATE TABLE "Bar" (
            "id" INTEGER NOT NULL,
        
            CONSTRAINT "Bar_pkey" PRIMARY KEY ("id")
        );
        
        CREATE UNIQUE INDEX "Foo_bar_id_key" ON "Foo"("bar_id");
    "#};

    api.raw_cmd(init).await;

    let input = indoc! {r#"
        generator client {
            provider = "prisma-client"
        }

        datasource db {
            provider     = "postgres"
            url          = "dummy-url"
            relationMode = "prisma"
        }

        model Foo {
            id     Int @id
            bar    Bar @relation(fields: [bar_id], references: [id])
            bar_id Int @unique
        }

        model Bar {
            id  Int  @id
            foo Foo?
        }
    "#};

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider     = "postgres"
          url          = "dummy-url"
          relationMode = "prisma"
        }

        model Foo {
          id     Int @id
          bar_id Int @unique
          bar    Bar @relation(fields: [bar_id], references: [id])
        }

        model Bar {
          id  Int  @id
          foo Foo?
        }
    "#]];

    let result = api.re_introspect_config(input).await?;
    expected.assert_eq(&result);

    Ok(())
}

// relationMode="foreignKeys" preserves the relation policy ("foreignKeys") as well as @relations, which are moved to the bottom.
#[test_connector(tags(Postgres))]
async fn relation_mode_foreign_keys(api: &mut TestApi) -> TestResult {
    let init = indoc! {r#"
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            "bar_id" INTEGER NOT NULL,
        
            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
        
        CREATE TABLE "Bar" (
            "id" INTEGER NOT NULL,
        
            CONSTRAINT "Bar_pkey" PRIMARY KEY ("id")
        );
        
        CREATE UNIQUE INDEX "Foo_bar_id_key" ON "Foo"("bar_id");

        ALTER TABLE "Foo" ADD CONSTRAINT "Foo_bar_id_fkey" FOREIGN KEY ("bar_id") REFERENCES "Bar"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
    "#};

    api.raw_cmd(init).await;

    let input = indoc! {r#"
        generator client {
            provider = "prisma-client"
        }

        datasource db {
            provider     = "postgres"
            url          = "dummy-url"
            relationMode = "foreignKeys"
        }

        model Foo {
            id     Int @id
            bar    Bar @relation(fields: [bar_id], references: [id])
            bar_id Int @unique
        }

        model Bar {
            id  Int  @id
            foo Foo?
        }
    "#};

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider     = "postgres"
          url          = "dummy-url"
          relationMode = "foreignKeys"
        }

        model Foo {
          id     Int @id
          bar_id Int @unique
          bar    Bar @relation(fields: [bar_id], references: [id])
        }

        model Bar {
          id  Int  @id
          foo Foo?
        }
    "#]];

    let result = api.re_introspect_config(input).await?;
    expected.assert_eq(&result);

    Ok(())
}

// @@map
mod at_at_map {
    use indoc::indoc;
    use sql_introspection_tests::test_api::*;

    // referentialIntegrity="prisma" is renamed as relationMode="prisma", and @relations are preserved.
    #[test_connector(tags(Postgres))]
    async fn referential_integrity_prisma_at_map_map(api: &mut TestApi) -> TestResult {
        let init = indoc! {r#"
            CREATE TABLE "foo_table" (
                "id" INTEGER NOT NULL,
                "bar_id" INTEGER NOT NULL,

                CONSTRAINT "foo_table_pkey" PRIMARY KEY ("id")
            );

            CREATE TABLE "bar_table" (
                "id" INTEGER NOT NULL,

                CONSTRAINT "bar_table_pkey" PRIMARY KEY ("id")
            );

            CREATE UNIQUE INDEX "foo_table_bar_id_key" ON "foo_table"("bar_id");
        "#};

        api.raw_cmd(init).await;

        let input = indoc! {r#"
            generator client {
                provider = "prisma-client"
            }

            datasource db {
                provider             = "postgres"
                url                  = "dummy-url"
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
              provider = "prisma-client"
            }

            datasource db {
              provider     = "postgres"
              url          = "dummy-url"
              relationMode = "prisma"
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

    // referentialIntegrity="foreignKeys" is renamed as relationMode="foreignKeys", and @relations are preserved.
    #[test_connector(tags(Postgres))]
    async fn referential_integrity_foreign_keys_at_map_map(api: &mut TestApi) -> TestResult {
        let init = indoc! {r#"
            CREATE TABLE "foo_table" (
                "id" INTEGER NOT NULL,
                "bar_id" INTEGER NOT NULL,

                CONSTRAINT "foo_table_pkey" PRIMARY KEY ("id")
            );

            CREATE TABLE "bar_table" (
                "id" INTEGER NOT NULL,

                CONSTRAINT "bar_table_pkey" PRIMARY KEY ("id")
            );

            CREATE UNIQUE INDEX "foo_table_bar_id_key" ON "foo_table"("bar_id");

            ALTER TABLE "foo_table" ADD CONSTRAINT "foo_table_bar_id_fkey" FOREIGN KEY ("bar_id") REFERENCES "bar_table"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
        "#};

        api.raw_cmd(init).await;

        let input = indoc! {r#"
            generator client {
                provider = "prisma-client"
            }

            datasource db {
                provider             = "postgres"
                url                  = "dummy-url"
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
              provider = "prisma-client"
            }

            datasource db {
              provider     = "postgres"
              url          = "dummy-url"
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

    // relationMode="prisma" preserves the relation policy ("prisma") as well as @relations.
    #[test_connector(tags(Postgres))]
    async fn relation_mode_prisma_at_map_map(api: &mut TestApi) -> TestResult {
        let init = indoc! {r#"
            CREATE TABLE "foo_table" (
                "id" INTEGER NOT NULL,
                "bar_id" INTEGER NOT NULL,

                CONSTRAINT "foo_table_pkey" PRIMARY KEY ("id")
            );

            CREATE TABLE "bar_table" (
                "id" INTEGER NOT NULL,

                CONSTRAINT "bar_table_pkey" PRIMARY KEY ("id")
            );

            CREATE UNIQUE INDEX "foo_table_bar_id_key" ON "foo_table"("bar_id");
        "#};

        api.raw_cmd(init).await;

        let input = indoc! {r#"
            generator client {
                provider = "prisma-client"
            }

            datasource db {
                provider     = "postgres"
                url          = "dummy-url"
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
              provider = "prisma-client"
            }

            datasource db {
              provider     = "postgres"
              url          = "dummy-url"
              relationMode = "prisma"
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

    // relationMode="foreignKeys" preserves the relation policy ("foreignKeys") as well as @relations., which are moved to the bottom.
    #[test_connector(tags(Postgres))]
    async fn relation_mode_foreign_keys_at_map_map(api: &mut TestApi) -> TestResult {
        let init = indoc! {r#"
            CREATE TABLE "foo_table" (
                "id" INTEGER NOT NULL,
                "bar_id" INTEGER NOT NULL,

                CONSTRAINT "foo_table_pkey" PRIMARY KEY ("id")
            );

            CREATE TABLE "bar_table" (
                "id" INTEGER NOT NULL,

                CONSTRAINT "bar_table_pkey" PRIMARY KEY ("id")
            );

            CREATE UNIQUE INDEX "foo_table_bar_id_key" ON "foo_table"("bar_id");

            ALTER TABLE "foo_table" ADD CONSTRAINT "foo_table_bar_id_fkey" FOREIGN KEY ("bar_id") REFERENCES "bar_table"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
        "#};

        api.raw_cmd(init).await;

        let input = indoc! {r#"
            generator client {
                provider = "prisma-client"
            }

            datasource db {
                provider     = "postgres"
                url          = "dummy-url"
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
              provider = "prisma-client"
            }

            datasource db {
              provider     = "postgres"
              url          = "dummy-url"
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
}
