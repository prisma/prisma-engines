#![allow(dead_code)]

use indoc::indoc;
use sql_migration_tests::test_api::*;

#[test_connector]
fn basic_create_migration_works(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    let is_postgres = api.is_postgres();
    let is_mysql = api.is_mysql();
    let is_sqlite = api.is_sqlite();
    let is_cockroach = api.is_cockroach();
    let is_mssql = api.is_mssql();

    api.create_migration("create-cats", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", move |migration| {
            let expected_script = if is_cockroach {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE "Cat" (
                        "id" INT4 NOT NULL,
                        "name" STRING NOT NULL,

                        CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                    );
                "#]]
            } else if is_postgres {
                expect![[r#"
                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" INTEGER NOT NULL,
                            "name" TEXT NOT NULL,

                            CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                        );
                    "#]]
            } else if is_mysql {
                expect![[r#"
                        -- CreateTable
                        CREATE TABLE `Cat` (
                            `id` INTEGER NOT NULL,
                            `name` VARCHAR(191) NOT NULL,

                            PRIMARY KEY (`id`)
                        ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
                        "#]]
            } else if is_sqlite {
                expect![[r#"
                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                            "name" TEXT NOT NULL
                        );
                        "#]]
            } else if is_mssql {
                expect![[r#"
                    BEGIN TRY

                    BEGIN TRAN;

                    -- CreateTable
                    CREATE TABLE [dbo].[Cat] (
                        [id] INT NOT NULL,
                        [name] NVARCHAR(1000) NOT NULL,
                        CONSTRAINT [Cat_pkey] PRIMARY KEY CLUSTERED ([id])
                    );

                    COMMIT TRAN;

                    END TRY
                    BEGIN CATCH

                    IF @@TRANCOUNT > 0
                    BEGIN
                        ROLLBACK TRAN;
                    END;
                    THROW

                    END CATCH
                "#]]
            } else {
                unreachable!()
            };

            migration.expect_contents(expected_script)
        });
}

#[test_connector]
fn basic_create_migration_multi_file_works(api: TestApi) {
    let schema_a = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let schema_b = indoc::indoc! {r#"
        model Dog {
            id Int @id
            name String
        }
    "#};

    let dir = api.create_migrations_directory();

    let is_postgres = api.is_postgres();
    let is_mysql = api.is_mysql();
    let is_sqlite = api.is_sqlite();
    let is_cockroach = api.is_cockroach();
    let is_mssql = api.is_mssql();

    api.create_migration_multi_file("create-cats", &[("a.prisma", &schema_a), ("b.prisma", schema_b)], &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", move |migration| {
            let expected_script = if is_cockroach {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE "Cat" (
                        "id" INT4 NOT NULL,
                        "name" STRING NOT NULL,

                        CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                    );

                    -- CreateTable
                    CREATE TABLE "Dog" (
                        "id" INT4 NOT NULL,
                        "name" STRING NOT NULL,

                        CONSTRAINT "Dog_pkey" PRIMARY KEY ("id")
                    );
                "#]]
            } else if is_postgres {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE "Cat" (
                        "id" INTEGER NOT NULL,
                        "name" TEXT NOT NULL,

                        CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                    );

                    -- CreateTable
                    CREATE TABLE "Dog" (
                        "id" INTEGER NOT NULL,
                        "name" TEXT NOT NULL,

                        CONSTRAINT "Dog_pkey" PRIMARY KEY ("id")
                    );
                "#]]
            } else if is_mysql {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE `Cat` (
                        `id` INTEGER NOT NULL,
                        `name` VARCHAR(191) NOT NULL,

                        PRIMARY KEY (`id`)
                    ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

                    -- CreateTable
                    CREATE TABLE `Dog` (
                        `id` INTEGER NOT NULL,
                        `name` VARCHAR(191) NOT NULL,

                        PRIMARY KEY (`id`)
                    ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
                "#]]
            } else if is_sqlite {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE "Cat" (
                        "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                        "name" TEXT NOT NULL
                    );

                    -- CreateTable
                    CREATE TABLE "Dog" (
                        "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                        "name" TEXT NOT NULL
                    );
                "#]]
            } else if is_mssql {
                expect![[r#"
                    BEGIN TRY

                    BEGIN TRAN;

                    -- CreateTable
                    CREATE TABLE [dbo].[Cat] (
                        [id] INT NOT NULL,
                        [name] NVARCHAR(1000) NOT NULL,
                        CONSTRAINT [Cat_pkey] PRIMARY KEY CLUSTERED ([id])
                    );

                    -- CreateTable
                    CREATE TABLE [dbo].[Dog] (
                        [id] INT NOT NULL,
                        [name] NVARCHAR(1000) NOT NULL,
                        CONSTRAINT [Dog_pkey] PRIMARY KEY CLUSTERED ([id])
                    );

                    COMMIT TRAN;

                    END TRY
                    BEGIN CATCH

                    IF @@TRANCOUNT > 0
                    BEGIN
                        ROLLBACK TRAN;
                    END;
                    THROW

                    END CATCH
                "#]]
            } else {
                unreachable!()
            };

            migration.expect_contents(expected_script)
        });
}

#[test_connector]
fn creating_a_second_migration_should_have_the_previous_sql_schema_as_baseline(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", &dm1, &dir)
        .send_sync()
        .assert_migration_directories_count(1);

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }

        model Dog {
            id      Int @id
            name    String
        }
    "#,
    );

    let is_postgres = api.is_postgres();
    let is_cockroach = api.is_cockroach();
    let is_mysql = api.is_mysql();
    let is_sqlite = api.is_sqlite();
    let is_mssql = api.is_mssql();
    api.create_migration("create-dogs", &dm2, &dir)
        .send_sync()
        .assert_migration_directories_count(2)
        .assert_migration("create-dogs", |migration| {
            let expected_script = if is_cockroach {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE "Dog" (
                        "id" INT4 NOT NULL,
                        "name" STRING NOT NULL,

                        CONSTRAINT "Dog_pkey" PRIMARY KEY ("id")
                    );
                "#]]
            } else if is_postgres {
                expect![[r#"
                        -- CreateTable
                        CREATE TABLE "Dog" (
                            "id" INTEGER NOT NULL,
                            "name" TEXT NOT NULL,

                            CONSTRAINT "Dog_pkey" PRIMARY KEY ("id")
                        );
                        "#]]
            } else if is_mysql {
                expect![[r#"
                        -- CreateTable
                        CREATE TABLE `Dog` (
                            `id` INTEGER NOT NULL,
                            `name` VARCHAR(191) NOT NULL,

                            PRIMARY KEY (`id`)
                        ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
                        "#]]
            } else if is_sqlite {
                expect![[r#"
                        -- CreateTable
                        CREATE TABLE "Dog" (
                            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                            "name" TEXT NOT NULL
                        );
                        "#]]
            } else if is_mssql {
                expect![[r#"
                        BEGIN TRY

                        BEGIN TRAN;

                        -- CreateTable
                        CREATE TABLE [dbo].[Dog] (
                            [id] INT NOT NULL,
                            [name] NVARCHAR(1000) NOT NULL,
                            CONSTRAINT [Dog_pkey] PRIMARY KEY CLUSTERED ([id])
                        );

                        COMMIT TRAN;

                        END TRY
                        BEGIN CATCH

                        IF @@TRANCOUNT > 0
                        BEGIN
                            ROLLBACK TRAN;
                        END;
                        THROW

                        END CATCH
                    "#]]
            } else {
                unreachable!()
            };

            migration.expect_contents(expected_script)
        });
}

#[test_connector]
fn bad_migrations_should_make_the_command_fail_with_a_good_error(api: TestApi) {
    use std::io::Write as _;

    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    let migration_directory = dir.path().join("20200916161900_broken-migration");
    std::fs::create_dir(&migration_directory).unwrap();
    let migration_file_path = migration_directory.join("migration.sql");
    let script = "this is not valid SQL";
    let mut file = std::fs::File::create(migration_file_path).unwrap();
    write!(file, "{script}").unwrap();

    let error = api.create_migration("create-cats", &dm, &dir).send_unwrap_err();

    assert!(error.to_string().contains("syntax"), "{}", error);
}

#[test_connector]
fn empty_migrations_should_not_be_created(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);

    api.create_migration("create-cats-again", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);
}

#[test_connector]
fn migration_name_length_is_validated(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("a-migration-with-a-name-that-is-way-too-long-a-migration-with-a-name-that-is-way-too-long-a-migration-with-a-name-that-is-way-too-long-a-migration-with-a-name-that-is-way-too-long", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);
}

#[test_connector]
fn empty_migrations_should_be_created_with_the_draft_option(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);

    api.create_migration("create-cats-again", &dm, &dir)
        .draft(true)
        .send_sync()
        .assert_migration_directories_count(2)
        .assert_migration("create-cats-again", |migration| {
            migration.assert_contents("-- This is an empty migration.")
        });
}

#[test_connector]
fn creating_a_migration_with_a_non_existent_migrations_directory_should_work(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    std::fs::remove_dir_all(dir.path()).unwrap();

    api.create_migration("create-cats", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);
}

#[test_connector(tags(Mysql, Postgres))]
fn create_enum_step_only_rendered_when_needed(api: TestApi) {
    let dm = format!(
        r#"
        {}

        model Cat {{
            id      Int @id
            mood    Mood
        }}

        enum Mood {{
            HUNGRY
            SLEEPY
        }}
    "#,
        api.datasource_block()
    );

    let dir = api.create_migrations_directory();

    let is_postgres = api.is_postgres();
    let is_cockroach = api.is_cockroach();
    let is_mysql = api.is_mysql();
    api.create_migration("create-cats", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", |migration| {
            let expected_script = if is_cockroach {
                indoc! {
                    r#"
                        -- CreateEnum
                        CREATE TYPE "Mood" AS ENUM ('HUNGRY', 'SLEEPY');

                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" INT4 NOT NULL,
                            "mood" "Mood" NOT NULL,

                            CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                        );
                    "#
                }
            } else if is_postgres {
                indoc! {
                    r#"
                        -- CreateEnum
                        CREATE TYPE "Mood" AS ENUM ('HUNGRY', 'SLEEPY');

                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" INTEGER NOT NULL,
                            "mood" "Mood" NOT NULL,

                            CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                        );
                    "#
                }
            } else if is_mysql {
                indoc! {
                    r#"
                        -- CreateTable
                        CREATE TABLE `Cat` (
                            `id` INTEGER NOT NULL,
                            `mood` ENUM('HUNGRY', 'SLEEPY') NOT NULL,

                            PRIMARY KEY (`id`)
                        ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
                        "#
                }
            } else {
                unreachable!("no enums -.-")
            };

            migration.assert_contents(expected_script)
        });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn create_enum_renders_correctly(api: TestApi) {
    let dm = r#"
        datasource test {
          provider = "postgresql"
          url = "postgresql://unreachable:unreachable@example.com/unreachable"
        }

        model Cat {
            id      Int @id
            mood    Mood
        }

        enum Mood{
            HUNGRY
            SLEEPY
        }
    "#;

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", |migration| {
            let expected_script = indoc! {
                    r#"
                        -- CreateEnum
                        CREATE TYPE "Mood" AS ENUM ('HUNGRY', 'SLEEPY');

                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" INTEGER NOT NULL,
                            "mood" "Mood" NOT NULL,

                            CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                        );
                    "#
            };

            migration.assert_contents(expected_script)
        });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn unsupported_type_renders_correctly(api: TestApi) {
    let dm = r#"
        datasource test {
          provider = "postgresql"
          url = "postgresql://unreachable:unreachable@example.com/unreachable"
        }

        model Cat {
            id      String @id
            home    Unsupported("point")
        }
    "#;

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", |migration| {
            let expected_script = indoc! {
                r#"
                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" TEXT NOT NULL,
                            "home" point NOT NULL,

                            CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                        );
                        "#
            };

            migration.assert_contents(expected_script)
        });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn no_additional_unique_created(api: TestApi) {
    let dm = r#"
        datasource test {
          provider = "postgresql"
          url = "postgresql://unreachable:unreachable@example.com/unreachable"
        }

        model Cat {
            id      Int @id
            collar  Collar?
        }

        model Collar {
            id      Int @id
            cat     Cat @relation(fields:[id], references: [id])
        }
    "#;

    let dir = api.create_migrations_directory();

    let is_postgres = api.is_postgres();
    api.create_migration("create-cats", dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", |migration| {
            let expected_script = if is_postgres {

                    indoc! {
                        r#"
                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" INTEGER NOT NULL,

                            CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                        );

                        -- CreateTable
                        CREATE TABLE "Collar" (
                            "id" INTEGER NOT NULL,

                            CONSTRAINT "Collar_pkey" PRIMARY KEY ("id")
                        );

                        -- AddForeignKey
                        ALTER TABLE "Collar" ADD CONSTRAINT "Collar_id_fkey" FOREIGN KEY ("id") REFERENCES "Cat"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
                        "#
                    }
                }
                else { unreachable!()
            };

            migration.assert_contents(expected_script)
        });
}

#[test_connector(exclude(Vitess))]
fn create_constraint_name_tests_w_implicit_names(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
         model A {
           id   Int    @id
           name String @unique
           a    String
           b    String
           B    B[]    @relation("AtoB")
           @@unique([a, b], name: "compound")
           @@index([a])
         }
         model B {
           a   String
           b   String
           aId Int
           A   A      @relation("AtoB", fields: [aId], references: [id])
           @@index([a,b])
           @@id([a, b])
         }
     "#,
    );

    let dir = api.create_migrations_directory();

    let is_mssql = api.is_mssql();
    let is_postgres = api.is_postgres();
    let is_mysql = api.is_mysql();
    let is_sqlite = api.is_sqlite();
    let is_cockroach = api.is_cockroach();
    api.create_migration("setup", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("setup", |migration| {
            let expected_script = if is_mssql {
                expect![[r#"
                    BEGIN TRY

                    BEGIN TRAN;

                    -- CreateTable
                    CREATE TABLE [dbo].[A] (
                        [id] INT NOT NULL,
                        [name] NVARCHAR(1000) NOT NULL,
                        [a] NVARCHAR(1000) NOT NULL,
                        [b] NVARCHAR(1000) NOT NULL,
                        CONSTRAINT [A_pkey] PRIMARY KEY CLUSTERED ([id]),
                        CONSTRAINT [A_name_key] UNIQUE NONCLUSTERED ([name]),
                        CONSTRAINT [A_a_b_key] UNIQUE NONCLUSTERED ([a],[b])
                    );

                    -- CreateTable
                    CREATE TABLE [dbo].[B] (
                        [a] NVARCHAR(1000) NOT NULL,
                        [b] NVARCHAR(1000) NOT NULL,
                        [aId] INT NOT NULL,
                        CONSTRAINT [B_pkey] PRIMARY KEY CLUSTERED ([a],[b])
                    );

                    -- CreateIndex
                    CREATE NONCLUSTERED INDEX [A_a_idx] ON [dbo].[A]([a]);

                    -- CreateIndex
                    CREATE NONCLUSTERED INDEX [B_a_b_idx] ON [dbo].[B]([a], [b]);

                    -- AddForeignKey
                    ALTER TABLE [dbo].[B] ADD CONSTRAINT [B_aId_fkey] FOREIGN KEY ([aId]) REFERENCES [dbo].[A]([id]) ON DELETE NO ACTION ON UPDATE CASCADE;

                    COMMIT TRAN;

                    END TRY
                    BEGIN CATCH

                    IF @@TRANCOUNT > 0
                    BEGIN
                        ROLLBACK TRAN;
                    END;
                    THROW

                    END CATCH
                "#]]
            } else if is_cockroach {
                expect![[
                     r#"
                    -- CreateTable
                    CREATE TABLE "A" (
                        "id" INT4 NOT NULL,
                        "name" STRING NOT NULL,
                        "a" STRING NOT NULL,
                        "b" STRING NOT NULL,

                        CONSTRAINT "A_pkey" PRIMARY KEY ("id")
                    );

                    -- CreateTable
                    CREATE TABLE "B" (
                        "a" STRING NOT NULL,
                        "b" STRING NOT NULL,
                        "aId" INT4 NOT NULL,

                        CONSTRAINT "B_pkey" PRIMARY KEY ("a","b")
                    );

                    -- CreateIndex
                    CREATE UNIQUE INDEX "A_name_key" ON "A"("name");

                    -- CreateIndex
                    CREATE INDEX "A_a_idx" ON "A"("a");

                    -- CreateIndex
                    CREATE UNIQUE INDEX "A_a_b_key" ON "A"("a", "b");

                    -- CreateIndex
                    CREATE INDEX "B_a_b_idx" ON "B"("a", "b");

                    -- AddForeignKey
                    ALTER TABLE "B" ADD CONSTRAINT "B_aId_fkey" FOREIGN KEY ("aId") REFERENCES "A"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
                "#
                     ]]
            } else if is_postgres {
                expect![[
                     r#"
                     -- CreateTable
                     CREATE TABLE "A" (
                         "id" INTEGER NOT NULL,
                         "name" TEXT NOT NULL,
                         "a" TEXT NOT NULL,
                         "b" TEXT NOT NULL,

                         CONSTRAINT "A_pkey" PRIMARY KEY ("id")
                     );

                     -- CreateTable
                     CREATE TABLE "B" (
                         "a" TEXT NOT NULL,
                         "b" TEXT NOT NULL,
                         "aId" INTEGER NOT NULL,

                         CONSTRAINT "B_pkey" PRIMARY KEY ("a","b")
                     );

                     -- CreateIndex
                     CREATE UNIQUE INDEX "A_name_key" ON "A"("name");

                     -- CreateIndex
                     CREATE INDEX "A_a_idx" ON "A"("a");

                     -- CreateIndex
                     CREATE UNIQUE INDEX "A_a_b_key" ON "A"("a", "b");

                     -- CreateIndex
                     CREATE INDEX "B_a_b_idx" ON "B"("a", "b");

                     -- AddForeignKey
                     ALTER TABLE "B" ADD CONSTRAINT "B_aId_fkey" FOREIGN KEY ("aId") REFERENCES "A"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
                 "#
                     ]]
            } else if is_mysql {
                expect![[
                     r#"
                 -- CreateTable
                 CREATE TABLE `A` (
                     `id` INTEGER NOT NULL,
                     `name` VARCHAR(191) NOT NULL,
                     `a` VARCHAR(191) NOT NULL,
                     `b` VARCHAR(191) NOT NULL,

                     UNIQUE INDEX `A_name_key`(`name`),
                     INDEX `A_a_idx`(`a`),
                     UNIQUE INDEX `A_a_b_key`(`a`, `b`),
                     PRIMARY KEY (`id`)
                 ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

                 -- CreateTable
                 CREATE TABLE `B` (
                     `a` VARCHAR(191) NOT NULL,
                     `b` VARCHAR(191) NOT NULL,
                     `aId` INTEGER NOT NULL,

                     INDEX `B_a_b_idx`(`a`, `b`),
                     PRIMARY KEY (`a`, `b`)
                 ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

                 -- AddForeignKey
                 ALTER TABLE `B` ADD CONSTRAINT `B_aId_fkey` FOREIGN KEY (`aId`) REFERENCES `A`(`id`) ON DELETE RESTRICT ON UPDATE CASCADE;
                 "#
                     ]]
            }else if is_sqlite {
                expect![[r#"
                 -- CreateTable
                 CREATE TABLE "A" (
                     "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                     "name" TEXT NOT NULL,
                     "a" TEXT NOT NULL,
                     "b" TEXT NOT NULL
                 );

                 -- CreateTable
                 CREATE TABLE "B" (
                     "a" TEXT NOT NULL,
                     "b" TEXT NOT NULL,
                     "aId" INTEGER NOT NULL,

                     PRIMARY KEY ("a", "b"),
                     CONSTRAINT "B_aId_fkey" FOREIGN KEY ("aId") REFERENCES "A" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
                 );

                 -- CreateIndex
                 CREATE UNIQUE INDEX "A_name_key" ON "A"("name");

                 -- CreateIndex
                 CREATE INDEX "A_a_idx" ON "A"("a");

                 -- CreateIndex
                 CREATE UNIQUE INDEX "A_a_b_key" ON "A"("a", "b");

                 -- CreateIndex
                 CREATE INDEX "B_a_b_idx" ON "B"("a", "b");
                 "#
                     ]]
             } else {
                 unreachable!();
            };

            migration.expect_contents(expected_script)
        });
}

#[test_connector(exclude(Sqlite, Vitess))]
fn create_constraint_name_tests_w_explicit_names(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
         model A {
           id   Int    @id
           name String @unique(map: "SingleUnique")
           a    String
           b    String
           B    B[]    @relation("AtoB")
           @@unique([a, b], name: "compound", map:"NamedCompoundUnique")
           @@unique([a, b], map:"UnNamedCompoundUnique")
           @@index([a], map: "SingleIndex")
         }

         model B {
           a   String
           b   String
           aId Int
           A   A      @relation("AtoB", fields: [aId], references: [id], map: "ForeignKey")
           @@index([a,b], map: "CompoundIndex")
           @@id([a, b])
         }
     "#,
    );

    let dir = api.create_migrations_directory();

    let is_mssql = api.is_mssql();
    let is_mysql = api.is_mysql();
    let is_postgres = api.is_postgres();
    let is_cockroach = api.is_cockroach();
    api.create_migration("setup", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("setup", move |migration| {
            let expected_script = if is_mssql {
                expect![[r#"
                    BEGIN TRY

                    BEGIN TRAN;

                    -- CreateTable
                    CREATE TABLE [dbo].[A] (
                        [id] INT NOT NULL,
                        [name] NVARCHAR(1000) NOT NULL,
                        [a] NVARCHAR(1000) NOT NULL,
                        [b] NVARCHAR(1000) NOT NULL,
                        CONSTRAINT [A_pkey] PRIMARY KEY CLUSTERED ([id]),
                        CONSTRAINT [SingleUnique] UNIQUE NONCLUSTERED ([name]),
                        CONSTRAINT [NamedCompoundUnique] UNIQUE NONCLUSTERED ([a],[b]),
                        CONSTRAINT [UnNamedCompoundUnique] UNIQUE NONCLUSTERED ([a],[b])
                    );

                    -- CreateTable
                    CREATE TABLE [dbo].[B] (
                        [a] NVARCHAR(1000) NOT NULL,
                        [b] NVARCHAR(1000) NOT NULL,
                        [aId] INT NOT NULL,
                        CONSTRAINT [B_pkey] PRIMARY KEY CLUSTERED ([a],[b])
                    );

                    -- CreateIndex
                    CREATE NONCLUSTERED INDEX [SingleIndex] ON [dbo].[A]([a]);

                    -- CreateIndex
                    CREATE NONCLUSTERED INDEX [CompoundIndex] ON [dbo].[B]([a], [b]);

                    -- AddForeignKey
                    ALTER TABLE [dbo].[B] ADD CONSTRAINT [ForeignKey] FOREIGN KEY ([aId]) REFERENCES [dbo].[A]([id]) ON DELETE NO ACTION ON UPDATE CASCADE;

                    COMMIT TRAN;

                    END TRY
                    BEGIN CATCH

                    IF @@TRANCOUNT > 0
                    BEGIN
                        ROLLBACK TRAN;
                    END;
                    THROW

                    END CATCH
                "#]]
            } else if is_cockroach {
                expect![[
                     r#"
                    -- CreateTable
                    CREATE TABLE "A" (
                        "id" INT4 NOT NULL,
                        "name" STRING NOT NULL,
                        "a" STRING NOT NULL,
                        "b" STRING NOT NULL,

                        CONSTRAINT "A_pkey" PRIMARY KEY ("id")
                    );

                    -- CreateTable
                    CREATE TABLE "B" (
                        "a" STRING NOT NULL,
                        "b" STRING NOT NULL,
                        "aId" INT4 NOT NULL,

                        CONSTRAINT "B_pkey" PRIMARY KEY ("a","b")
                    );

                    -- CreateIndex
                    CREATE UNIQUE INDEX "SingleUnique" ON "A"("name");

                    -- CreateIndex
                    CREATE INDEX "SingleIndex" ON "A"("a");

                    -- CreateIndex
                    CREATE UNIQUE INDEX "NamedCompoundUnique" ON "A"("a", "b");

                    -- CreateIndex
                    CREATE UNIQUE INDEX "UnNamedCompoundUnique" ON "A"("a", "b");

                    -- CreateIndex
                    CREATE INDEX "CompoundIndex" ON "B"("a", "b");

                    -- AddForeignKey
                    ALTER TABLE "B" ADD CONSTRAINT "ForeignKey" FOREIGN KEY ("aId") REFERENCES "A"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
                "#
                     ]]
            }else if is_postgres {
                expect![[
                     r#"
                     -- CreateTable
                     CREATE TABLE "A" (
                         "id" INTEGER NOT NULL,
                         "name" TEXT NOT NULL,
                         "a" TEXT NOT NULL,
                         "b" TEXT NOT NULL,

                         CONSTRAINT "A_pkey" PRIMARY KEY ("id")
                     );

                     -- CreateTable
                     CREATE TABLE "B" (
                         "a" TEXT NOT NULL,
                         "b" TEXT NOT NULL,
                         "aId" INTEGER NOT NULL,

                         CONSTRAINT "B_pkey" PRIMARY KEY ("a","b")
                     );

                     -- CreateIndex
                     CREATE UNIQUE INDEX "SingleUnique" ON "A"("name");

                     -- CreateIndex
                     CREATE INDEX "SingleIndex" ON "A"("a");

                     -- CreateIndex
                     CREATE UNIQUE INDEX "NamedCompoundUnique" ON "A"("a", "b");

                     -- CreateIndex
                     CREATE UNIQUE INDEX "UnNamedCompoundUnique" ON "A"("a", "b");

                     -- CreateIndex
                     CREATE INDEX "CompoundIndex" ON "B"("a", "b");

                     -- AddForeignKey
                     ALTER TABLE "B" ADD CONSTRAINT "ForeignKey" FOREIGN KEY ("aId") REFERENCES "A"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
                 "#
                     ]]
            } else if is_mysql {
                expect![[
                     r#"
                 -- CreateTable
                 CREATE TABLE `A` (
                     `id` INTEGER NOT NULL,
                     `name` VARCHAR(191) NOT NULL,
                     `a` VARCHAR(191) NOT NULL,
                     `b` VARCHAR(191) NOT NULL,

                     UNIQUE INDEX `SingleUnique`(`name`),
                     INDEX `SingleIndex`(`a`),
                     UNIQUE INDEX `NamedCompoundUnique`(`a`, `b`),
                     UNIQUE INDEX `UnNamedCompoundUnique`(`a`, `b`),
                     PRIMARY KEY (`id`)
                 ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

                 -- CreateTable
                 CREATE TABLE `B` (
                     `a` VARCHAR(191) NOT NULL,
                     `b` VARCHAR(191) NOT NULL,
                     `aId` INTEGER NOT NULL,

                     INDEX `CompoundIndex`(`a`, `b`),
                     PRIMARY KEY (`a`, `b`)
                 ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

                 -- AddForeignKey
                 ALTER TABLE `B` ADD CONSTRAINT `ForeignKey` FOREIGN KEY (`aId`) REFERENCES `A`(`id`) ON DELETE RESTRICT ON UPDATE CASCADE;
                 "#
                     ]]
            } else {
                 unreachable!();
            };

            migration.expect_contents(expected_script)
        });
}

#[cfg_attr(not(target_os = "windows"), test_connector(exclude(Mysql)))]
fn alter_constraint_name(mut api: TestApi) {
    let plain_dm = api.datamodel_with_provider(
        r#"
         model A {
           id   Int    @id
           name String @unique
           a    String
           b    String
           B    B[]    @relation("AtoB")
           @@unique([a, b])
           @@index([a])
         }
         model B {
           a   String
           b   String
           aId Int
           A   A      @relation("AtoB", fields: [aId], references: [id])
           @@index([a,b])
           @@id([a, b])
         }
     "#,
    );

    let dir = api.create_migrations_directory();
    api.create_migration("plain", &plain_dm, &dir).send_sync();

    let custom_dm = api.datamodel_with_provider(&format!(
        r#"
         model A {{
           id   Int    @id{}
           name String @unique(map: "CustomUnique")
           a    String
           b    String
           B    B[]    @relation("AtoB")
           @@unique([a, b], name: "compound", map:"CustomCompoundUnique")
           @@index([a], map: "CustomIndex")
         }}

         model B {{
           a   String
           b   String
           aId Int
           A   A      @relation("AtoB", fields: [aId], references: [id]{})
           @@index([a,b], map: "AnotherCustomIndex")
           @@id([a, b]{})
         }}
     "#,
        if api.is_sqlite() { "" } else { r#"(map: "CustomId")"# },
        if api.is_sqlite() { "" } else { r#", map: "CustomFK""# },
        if api.is_sqlite() || api.is_mysql() {
            ""
        } else {
            r#", map: "CustomCompoundId""#
        }
    ));

    let is_mssql = api.is_mssql();
    let is_postgres = api.is_postgres();
    let is_postgres15 = api.is_postgres_15();
    let is_postgres16 = api.is_postgres_16();
    let is_cockroach = api.is_cockroach();
    let is_sqlite = api.is_sqlite();

    api.create_migration("custom", &custom_dm, &dir)
        .send_sync()
        .assert_migration_directories_count(2)
        .assert_migration("custom", move |migration| {
            let expected_script = if is_mssql {
                expect![[r#"
                    BEGIN TRY

                    BEGIN TRAN;

                    -- AlterTable
                    EXEC SP_RENAME N'dbo.A_pkey', N'CustomId';

                    -- AlterTable
                    EXEC SP_RENAME N'dbo.B_pkey', N'CustomCompoundId';

                    -- RenameForeignKey
                    EXEC sp_rename 'dbo.B_aId_fkey', 'CustomFK', 'OBJECT';

                    -- RenameIndex
                    EXEC SP_RENAME N'dbo.A.A_a_b_key', N'CustomCompoundUnique', N'INDEX';

                    -- RenameIndex
                    EXEC SP_RENAME N'dbo.A.A_a_idx', N'CustomIndex', N'INDEX';

                    -- RenameIndex
                    EXEC SP_RENAME N'dbo.A.A_name_key', N'CustomUnique', N'INDEX';

                    -- RenameIndex
                    EXEC SP_RENAME N'dbo.B.B_a_b_idx', N'AnotherCustomIndex', N'INDEX';

                    COMMIT TRAN;

                    END TRY
                    BEGIN CATCH

                    IF @@TRANCOUNT > 0
                    BEGIN
                        ROLLBACK TRAN;
                    END;
                    THROW

                    END CATCH
                "#]]
            } else if is_cockroach {
                expect![[r#"
                    -- AlterTable
                    ALTER TABLE "A" RENAME CONSTRAINT "A_pkey" TO "CustomId";

                    -- AlterTable
                    ALTER TABLE "B" RENAME CONSTRAINT "B_pkey" TO "CustomCompoundId";

                    -- RenameForeignKey
                    ALTER TABLE "B" RENAME CONSTRAINT "B_aId_fkey" TO "CustomFK";

                    -- RenameIndex
                    ALTER INDEX "A_a_b_key" RENAME TO "CustomCompoundUnique";

                    -- RenameIndex
                    ALTER INDEX "A_a_idx" RENAME TO "CustomIndex";

                    -- RenameIndex
                    ALTER INDEX "A_name_key" RENAME TO "CustomUnique";

                    -- RenameIndex
                    ALTER INDEX "B_a_b_idx" RENAME TO "AnotherCustomIndex";
                "#]]
            } else if is_postgres15 || is_postgres16 {
                expect![[r#"
                    -- AlterTable
                    ALTER TABLE "A" RENAME CONSTRAINT "A_pkey" TO "CustomId";

                    -- AlterTable
                    ALTER TABLE "B" RENAME CONSTRAINT "B_pkey" TO "CustomCompoundId";

                    -- RenameForeignKey
                    ALTER TABLE "B" RENAME CONSTRAINT "B_aId_fkey" TO "CustomFK";

                    -- RenameIndex
                    ALTER INDEX "A_a_b_key" RENAME TO "CustomCompoundUnique";

                    -- RenameIndex
                    ALTER INDEX "A_a_idx" RENAME TO "CustomIndex";

                    -- RenameIndex
                    ALTER INDEX "A_name_key" RENAME TO "CustomUnique";

                    -- RenameIndex
                    ALTER INDEX "B_a_b_idx" RENAME TO "AnotherCustomIndex";
                "#]]
            } else if is_postgres {
                expect![[r#"
                -- AlterTable
                ALTER TABLE "A" RENAME CONSTRAINT "A_pkey" TO "CustomId";

                -- AlterTable
                ALTER TABLE "B" RENAME CONSTRAINT "B_pkey" TO "CustomCompoundId";

                -- RenameForeignKey
                ALTER TABLE "B" RENAME CONSTRAINT "B_aId_fkey" TO "CustomFK";

                -- RenameIndex
                ALTER INDEX "A_a_b_key" RENAME TO "CustomCompoundUnique";

                -- RenameIndex
                ALTER INDEX "A_a_idx" RENAME TO "CustomIndex";

                -- RenameIndex
                ALTER INDEX "A_name_key" RENAME TO "CustomUnique";

                -- RenameIndex
                ALTER INDEX "B_a_b_idx" RENAME TO "AnotherCustomIndex";
                "#]]
            } else if is_sqlite {
                expect![[r#"
                 -- RedefineIndex
                 DROP INDEX "A_a_b_key";
                 CREATE UNIQUE INDEX "CustomCompoundUnique" ON "A"("a", "b");

                 -- RedefineIndex
                 DROP INDEX "A_a_idx";
                 CREATE INDEX "CustomIndex" ON "A"("a");

                 -- RedefineIndex
                 DROP INDEX "A_name_key";
                 CREATE UNIQUE INDEX "CustomUnique" ON "A"("name");

                 -- RedefineIndex
                 DROP INDEX "B_a_b_idx";
                 CREATE INDEX "AnotherCustomIndex" ON "B"("a", "b");
                 "#]]
            } else {
                panic!()
            };

            migration.expect_contents(expected_script)
        });
}
