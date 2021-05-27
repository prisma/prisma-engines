use indoc::indoc;
use migration_engine_tests::sync_test_api::*;

#[test_connector]
fn basic_create_migration_works(api: TestApi) {
    let dm = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", |migration| {
            let expected_script = if api.is_postgres() {
                indoc! {
                    r#"
                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" INTEGER NOT NULL,
                            "name" TEXT NOT NULL,

                            CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                        );
                        "#
                }
            } else if api.is_mysql() {
                indoc! {
                    r#"
                        -- CreateTable
                        CREATE TABLE `Cat` (
                            `id` INTEGER NOT NULL,
                            `name` VARCHAR(191) NOT NULL,

                            PRIMARY KEY (`id`)
                        ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
                        "#
                }
            } else if api.is_sqlite() {
                indoc! {
                    r#"
                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                            "name" TEXT NOT NULL
                        );
                        "#
                }
            } else if api.is_mssql() {
                indoc! {
                    r#"
                        -- CreateTable
                        CREATE TABLE [basic_create_migration_works].[Cat] (
                            [id] INT NOT NULL,
                            [name] NVARCHAR(1000) NOT NULL,
                        
                            CONSTRAINT [Cat_pkey] PRIMARY KEY ([id])
                        );
                        "#
                }
            } else {
                unreachable!()
            };

            migration.assert_contents(expected_script)
        });
}

#[test_connector]
fn creating_a_second_migration_should_have_the_previous_sql_schema_as_baseline(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", dm1, &dir)
        .send_sync()
        .assert_migration_directories_count(1);

    let dm2 = r#"
        model Cat {
            id      Int @id
            name    String
        }

        model Dog {
            id      Int @id
            name    String
        }
    "#;

    api.create_migration("create-dogs", dm2, &dir)
        .send_sync()

        .assert_migration_directories_count(2)
        .assert_migration("create-dogs", |migration| {
            let expected_script = if api.is_postgres()
                {
                    indoc! {
                        r#"
                        -- CreateTable
                        CREATE TABLE "Dog" (
                            "id" INTEGER NOT NULL,
                            "name" TEXT NOT NULL,

                            CONSTRAINT "Dog_pkey" PRIMARY KEY ("id")
                        );
                        "#
                    }
                }
                else if api.is_mysql() {
                    indoc! {
                        r#"
                        -- CreateTable
                        CREATE TABLE `Dog` (
                            `id` INTEGER NOT NULL,
                            `name` VARCHAR(191) NOT NULL,

                            PRIMARY KEY (`id`)
                        ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
                        "#
                    }
                }
                else if api.is_sqlite() {
                    indoc! {
                        r#"
                        -- CreateTable
                        CREATE TABLE "Dog" (
                            "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                            "name" TEXT NOT NULL
                        );
                        "#
                    }
                }
                else if api.is_mssql() {
                    indoc! {
                        r#"
                        -- CreateTable
                        CREATE TABLE [creating_a_second_migration_should_have_the_previous_sql_schema_as_baseline].[Dog] (
                            [id] INT NOT NULL,
                            [name] NVARCHAR(1000) NOT NULL,
                        
                            CONSTRAINT [Dog_pkey] PRIMARY KEY ([id])
                        );
                        "#
                    }
                } else {
                    unreachable!()
                };

            migration.assert_contents(expected_script)
        });
}

#[test_connector]
fn bad_migrations_should_make_the_command_fail_with_a_good_error(api: TestApi) {
    use std::io::Write as _;

    let dm = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let dir = api.create_migrations_directory();

    let migration_directory = dir.path().join("20200916161900_broken-migration");
    std::fs::create_dir(&migration_directory).unwrap();
    let migration_file_path = migration_directory.join("migration.sql");
    let script = "this is not valid SQL";
    let mut file = std::fs::File::create(&migration_file_path).unwrap();
    write!(file, "{}", script).unwrap();

    let error = api.create_migration("create-cats", dm, &dir).send_unwrap_err();

    assert!(error.to_string().contains("syntax"), "{}", error);
}

#[test_connector]
fn empty_migrations_should_not_be_created(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);

    api.create_migration("create-cats-again", dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);
}

#[test_connector]
fn migration_name_length_is_validated(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let dir = api.create_migrations_directory();

    api.create_migration("a-migration-with-a-name-that-is-way-too-long-a-migration-with-a-name-that-is-way-too-long-a-migration-with-a-name-that-is-way-too-long-a-migration-with-a-name-that-is-way-too-long", dm, &dir)
        .send_sync()

        .assert_migration_directories_count(1);
}

#[test_connector]
fn empty_migrations_should_be_created_with_the_draft_option(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);

    api.create_migration("create-cats-again", dm, &dir)
        .draft(true)
        .send_sync()
        .assert_migration_directories_count(2)
        .assert_migration("create-cats-again", |migration| {
            migration.assert_contents("-- This is an empty migration.")
        });
}

#[test_connector]
fn creating_a_migration_with_a_non_existent_migrations_directory_should_work(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let dir = api.create_migrations_directory();

    std::fs::remove_dir_all(&dir.path()).unwrap();

    api.create_migration("create-cats", dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1);
}

#[test_connector(tags(Mysql, Postgres))]
fn create_enum_step_only_rendered_when_needed(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            mood    Mood
        }

        enum Mood{
            HUNGRY
            SLEEPY
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", |migration| {
            let expected_script = if api.is_postgres() {
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
            } else if api.is_mysql() {
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

#[test_connector(tags(Postgres))]
fn create_enum_renders_correctly(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            mood    Mood
        }

        enum Mood{
            HUNGRY
            SLEEPY
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", |migration| {
            let expected_script = if api.is_postgres() {
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
            } else {
                unreachable!()
            };

            migration.assert_contents(expected_script)
        });
}

#[test_connector(tags(Postgres))]
fn unsupported_type_renders_correctly(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            home    Unsupported("point")
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", |migration| {
            let expected_script = if api.is_postgres() {
                indoc! {
                    r#"
                        -- CreateTable
                        CREATE TABLE "Cat" (
                            "id" INTEGER NOT NULL,
                            "home" point NOT NULL,

                            CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                        );
                        "#
                }
            } else {
                unreachable!()
            };

            migration.assert_contents(expected_script)
        });
}

#[test_connector(tags(Postgres))]
fn no_additional_unique_created(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            collar  Collar?
        }

        model Collar {
            id      Int @id
            cat     Cat @relation(fields:[id], references: [id])
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    api.create_migration("create-cats", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats", |migration| {
            let expected_script = if api.is_postgres() {

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
                        ALTER TABLE "Collar" ADD FOREIGN KEY ("id") REFERENCES "Cat"("id") ON DELETE CASCADE ON UPDATE CASCADE;
                        "#
                    }
                }
                else { unreachable!()
            };

            migration.assert_contents(expected_script)
        });
}
// todo just for testing, figure out whether to keep it
#[test_connector]
fn index_tests(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id
          name String @unique
          a    String
          b    String
          B    B[]    @relation("AtoB")

          @@unique([a, b], name: "compound", map:"1")
          @@unique([a, b], map:"2")
          @@index([a])
        }

        model B {
          a   String
          b   String
          aId Int
          A   A      @relation("AtoB", fields: [aId], references: [id])

          @@index([a,b])
          @@id([a, b], map: "custom")
        }
    "#;

    let dir = api.create_migrations_directory();

    api.create_migration("setup", dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("setup", |migration| {
            let expected_script = if api.is_mssql() {
                indoc! {
                    r#"
                    -- CreateTable
                    CREATE TABLE [index_tests].[A] (
                        [id] INT NOT NULL,
                        [name] NVARCHAR(1000) NOT NULL,
                        [a] NVARCHAR(1000) NOT NULL,
                        [b] NVARCHAR(1000) NOT NULL,
                    
                        CONSTRAINT [A_pkey] PRIMARY KEY ([id]),
                        CONSTRAINT [1] UNIQUE ([a],[b]),
                        CONSTRAINT [2] UNIQUE ([a],[b]),
                        CONSTRAINT [A_name_key] UNIQUE ([name])
                    );
                    
                    -- CreateTable
                    CREATE TABLE [index_tests].[B] (
                        [a] NVARCHAR(1000) NOT NULL,
                        [b] NVARCHAR(1000) NOT NULL,
                        [aId] INT NOT NULL,
                    
                        CONSTRAINT [custom] PRIMARY KEY ([a],[b])
                    );
                    
                    -- CreateIndex
                    CREATE INDEX [A_a_idx] ON [index_tests].[A]([a]);
                    
                    -- CreateIndex
                    CREATE INDEX [B_a_b_idx] ON [index_tests].[B]([a], [b]);
                    
                    -- AddForeignKey
                    ALTER TABLE [index_tests].[B] ADD CONSTRAINT [FK__B__aId] FOREIGN KEY ([aId]) REFERENCES [index_tests].[A]([id]) ON DELETE CASCADE ON UPDATE CASCADE;
                "#
                }
            } else if api.is_postgres() {
                indoc! {
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
                    
                        CONSTRAINT "custom" PRIMARY KEY ("a","b")
                    );
                    
                    -- CreateIndex
                    CREATE UNIQUE INDEX "1" ON "A"("a", "b");
                    
                    -- CreateIndex
                    CREATE UNIQUE INDEX "2" ON "A"("a", "b");
                    
                    -- CreateIndex
                    CREATE INDEX "A_a_idx" ON "A"("a");
                    
                    -- CreateIndex
                    CREATE UNIQUE INDEX "A_name_key" ON "A"("name");
                    
                    -- CreateIndex
                    CREATE INDEX "B_a_b_idx" ON "B"("a", "b");
                    
                    -- AddForeignKey
                    ALTER TABLE "B" ADD FOREIGN KEY ("aId") REFERENCES "A"("id") ON DELETE CASCADE ON UPDATE CASCADE;
                "#
                }
            } else if api.is_mysql(){
                indoc! {
                    r#"
                -- CreateTable
                CREATE TABLE `A` (
                    `id` INTEGER NOT NULL,
                    `name` VARCHAR(191) NOT NULL,
                    `a` VARCHAR(191) NOT NULL,
                    `b` VARCHAR(191) NOT NULL,
                    UNIQUE INDEX `1`(`a`, `b`),
                    UNIQUE INDEX `2`(`a`, `b`),
                    INDEX `A_a_idx`(`a`),
                    UNIQUE INDEX `A_name_key`(`name`),
                
                    PRIMARY KEY (`id`)
                ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
                
                -- CreateTable
                CREATE TABLE `B` (
                    `a` VARCHAR(191) NOT NULL,
                    `b` VARCHAR(191) NOT NULL,
                    `aId` INTEGER NOT NULL,
                    INDEX `B_a_b_idx`(`a`, `b`),
                
                    PRIMARY KEY (`a`,`b`)
                ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
                
                -- AddForeignKey
                ALTER TABLE `B` ADD FOREIGN KEY (`aId`) REFERENCES `A`(`id`) ON DELETE CASCADE ON UPDATE CASCADE;
                "#
                }
            }else if api.is_sqlite(){
                indoc!{r#"
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
                    FOREIGN KEY ("aId") REFERENCES "A" ("id") ON DELETE CASCADE ON UPDATE CASCADE
                );
                
                -- CreateIndex
                CREATE UNIQUE INDEX "1" ON "A"("a", "b");
                
                -- CreateIndex
                CREATE UNIQUE INDEX "2" ON "A"("a", "b");
                
                -- CreateIndex
                CREATE INDEX "A_a_idx" ON "A"("a");
                
                -- CreateIndex
                CREATE UNIQUE INDEX "A_name_key" ON "A"("name");
                
                -- CreateIndex
                CREATE INDEX "B_a_b_idx" ON "B"("a", "b");
                "#
            }} else {
                ""
            };

            migration.assert_contents(expected_script)
        });
}
