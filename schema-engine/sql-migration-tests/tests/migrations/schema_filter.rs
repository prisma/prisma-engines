use schema_core::json_rpc::types::SchemaFilter;
use sql_migration_tests::test_api::*;

#[test_connector]
fn schema_filter_migration_adding_external_table(api: TestApi) {
    let schema = api.datamodel_with_provider(
        r#"
        model ExternalTable {
            id      Int @id
            name    String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    let filter = SchemaFilter {
        external_tables: vec!["ExternalTable".to_string()],
    };
    api.create_migration_with_filter("custom", &schema, &dir, filter, "")
        .send_sync()
        .assert_migration_directories_count(0);
}

#[test_connector]
fn schema_filter_migration_removing_external_table(mut api: TestApi) {
    let schema_1 = api.datamodel_with_provider(
        r#"
        model ExternalTable {
            id      Int @id
            name    String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    // No filter applied here to actually create the external tables first
    api.create_migration("create", &schema_1, &dir).send_sync();

    let schema_2 = api.datamodel_with_provider("");

    let filter = SchemaFilter {
        external_tables: vec!["ExternalTable".to_string()],
    };
    api.create_migration_with_filter("remove", &schema_2, &dir, filter, "")
        .send_sync()
        .assert_migration_directories_count(1);
}

#[test_connector]
fn schema_filter_migration_removing_external_table_with_contents(mut api: TestApi) {
    let schema_1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    // No filter applied here to actually create the external tables first
    api.create_migration("create", &schema_1, &dir).send_sync();
    api.apply_migrations(&dir).send_sync();

    api.insert("Cat").value("id", 1).value("name", "Felix").result_raw();
    api.insert("Cat").value("id", 2).value("name", "Norbert").result_raw();

    let schema_2 = api.datamodel_with_provider("");

    let filter = SchemaFilter {
        external_tables: vec!["Cat".to_string()],
    };
    api.evaluate_data_loss_with_filter(&dir, schema_2.clone(), filter)
        .send()
        .assert_warnings(&[]);
}

#[test_connector]
fn schema_filter_migration_modifying_external_table(mut api: TestApi) {
    let schema_1 = api.datamodel_with_provider(
        r#"
        model ExternalTable {
            id      Int @id
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    // No filter applied here to actually create the external tables first
    api.create_migration("create", &schema_1, &dir).send_sync();

    let schema_2 = api.datamodel_with_provider(
        r#"
            model ExternalTable {
                id      Int @id
                name    String
            }
        "#,
    );

    let filter = SchemaFilter {
        external_tables: vec!["ExternalTable".to_string()],
    };
    api.create_migration_with_filter("modify", &schema_2, &dir, filter, "")
        .send_sync()
        .assert_migration_directories_count(1);
}

#[test_connector(exclude(CockroachDb, Vitess))]
fn schema_filter_migration_adding_external_tables_incl_relations(api: TestApi) {
    let schema = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
            
            // This relation SHOULD NOT be represented in the migration script
            externalTablesB ExternalTableB[]

            // This relation SHOULD be represented in the migration script
            externalTableA ExternalTableA? @relation(fields: [externalTableId], references: [id])
            externalTableId Int?
        }

        // This table SHOULD NOT be represented in the migration script
        model ExternalTableA {
            id      Int @id
            name    String
            cats    Cat[]
        }

        // This table SHOULD NOT be represented in the migration script
        model ExternalTableB {
            id      Int @id
            name    String
            cat     Cat? @relation(fields: [catId], references: [id])
            catId   Int?
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    let is_postgres = api.is_postgres();
    let is_mysql = api.is_mysql();
    let is_sqlite = api.is_sqlite();
    let is_mssql = api.is_mssql();

    let filter = SchemaFilter {
        external_tables: vec!["ExternalTableA".to_string(), "ExternalTableB".to_string()],
    };
    api.create_migration_with_filter("custom", &schema, &dir, filter, "")
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("custom", move |migration| {
            // migration contains no create table statements for external tables
            let expected_script = if is_postgres {
                expect![[r#"
                -- CreateTable
                CREATE TABLE "Cat" (
                    "id" INTEGER NOT NULL,
                    "name" TEXT NOT NULL,
                    "externalTableId" INTEGER,

                    CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                );

                -- AddForeignKey
                ALTER TABLE "Cat" ADD CONSTRAINT "Cat_externalTableId_fkey" FOREIGN KEY ("externalTableId") REFERENCES "ExternalTableA"("id") ON DELETE SET NULL ON UPDATE CASCADE;
            "#]]
            } else if is_mysql {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE `Cat` (
                        `id` INTEGER NOT NULL,
                        `name` VARCHAR(191) NOT NULL,
                        `externalTableId` INTEGER NULL,

                        PRIMARY KEY (`id`)
                    ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

                    -- AddForeignKey
                    ALTER TABLE `Cat` ADD CONSTRAINT `Cat_externalTableId_fkey` FOREIGN KEY (`externalTableId`) REFERENCES `ExternalTableA`(`id`) ON DELETE SET NULL ON UPDATE CASCADE;
                "#]]
            } else if is_sqlite {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE "Cat" (
                        "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                        "name" TEXT NOT NULL,
                        "externalTableId" INTEGER,
                        CONSTRAINT "Cat_externalTableId_fkey" FOREIGN KEY ("externalTableId") REFERENCES "ExternalTableA" ("id") ON DELETE SET NULL ON UPDATE CASCADE
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
                        [externalTableId] INT,
                        CONSTRAINT [Cat_pkey] PRIMARY KEY CLUSTERED ([id])
                    );

                    -- AddForeignKey
                    ALTER TABLE [dbo].[Cat] ADD CONSTRAINT [Cat_externalTableId_fkey] FOREIGN KEY ([externalTableId]) REFERENCES [dbo].[ExternalTableA]([id]) ON DELETE SET NULL ON UPDATE CASCADE;

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

#[test_connector(exclude(CockroachDb, Vitess))]
fn schema_filter_migration_removing_external_tables_incl_relations(mut api: TestApi) {
    let schema_1 = api.datamodel_with_provider(
        r#"
        model cat {
            id      Int @id
            name    String
            
            externalTablesB ExternalTableB[]

            externalTableA ExternalTableA? @relation(fields: [externalTableId], references: [id])
            externalTableId Int?
        }

        model ExternalTableA {
            id      Int @id
            name    String
            cats    cat[]
        }

        model ExternalTableB {
            id      Int @id
            name    String
            cat     cat? @relation(fields: [catId], references: [id])
            catId   Int?
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    let is_postgres = api.is_postgres();
    let is_mysql = api.is_mysql();
    let is_sqlite = api.is_sqlite();
    let is_mssql = api.is_mssql();

    // No filter applied here to actually create the external tables first
    api.create_migration("create", &schema_1, &dir).send_sync();

    let schema_2 = api.datamodel_with_provider(
        r#"
            model cat {
                id      Int @id
                name    String
            }
        "#,
    );

    let filter = SchemaFilter {
        external_tables: vec!["ExternalTableA".to_string(), "ExternalTableB".to_string()],
    };
    api.create_migration_with_filter("remove", &schema_2, &dir, filter, "")
        .send_sync()
        .assert_migration_directories_count(2)
        .assert_migration("remove", move |migration| {
            // migration contains no drop table statements for external tables
            let expected_script = if is_postgres {
                expect![[r#"
                /*
                  Warnings:

                  - You are about to drop the column `externalTableId` on the `cat` table. All the data in the column will be lost.

                */
                -- DropForeignKey
                ALTER TABLE "cat" DROP CONSTRAINT "cat_externalTableId_fkey";

                -- AlterTable
                ALTER TABLE "cat" DROP COLUMN "externalTableId";
            "#]]
            } else if is_mysql {
                expect![[r#"
                    /*
                      Warnings:

                      - You are about to drop the column `externalTableId` on the `cat` table. All the data in the column will be lost.

                    */
                    -- DropForeignKey
                    ALTER TABLE `cat` DROP FOREIGN KEY `cat_externalTableId_fkey`;

                    -- DropIndex
                    DROP INDEX `cat_externalTableId_fkey` ON `cat`;

                    -- AlterTable
                    ALTER TABLE `cat` DROP COLUMN `externalTableId`;
                "#]]
            } else if is_sqlite {
                expect![[r#"
                    /*
                      Warnings:

                      - You are about to drop the column `externalTableId` on the `cat` table. All the data in the column will be lost.

                    */
                    -- RedefineTables
                    PRAGMA defer_foreign_keys=ON;
                    PRAGMA foreign_keys=OFF;
                    CREATE TABLE "new_cat" (
                        "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                        "name" TEXT NOT NULL
                    );
                    INSERT INTO "new_cat" ("id", "name") SELECT "id", "name" FROM "cat";
                    DROP TABLE "cat";
                    ALTER TABLE "new_cat" RENAME TO "cat";
                    PRAGMA foreign_keys=ON;
                    PRAGMA defer_foreign_keys=OFF;
                "#]]
            } else if is_mssql {
                expect![[r#"
                    /*
                      Warnings:

                      - You are about to drop the column `externalTableId` on the `cat` table. All the data in the column will be lost.

                    */
                    BEGIN TRY

                    BEGIN TRAN;

                    -- DropForeignKey
                    ALTER TABLE [dbo].[cat] DROP CONSTRAINT [cat_externalTableId_fkey];

                    -- AlterTable
                    ALTER TABLE [dbo].[cat] DROP COLUMN [externalTableId];

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

#[test_connector(exclude(CockroachDb, Vitess))]
fn schema_filter_migration_modifying_external_tables_incl_relations(mut api: TestApi) {
    let schema_1 = api.datamodel_with_provider(
        r#"
        model cat {
            id      Int @id
            name    String
        }

        model ExternalTableA {
            id      Int @id
        }

        model ExternalTableB {
            id      Int @id
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    let is_postgres = api.is_postgres();
    let is_mysql = api.is_mysql();
    let is_sqlite = api.is_sqlite();
    let is_mssql = api.is_mssql();

    // No filter applied here to actually create the external tables first
    api.create_migration("create", &schema_1, &dir).send_sync();

    let schema_2 = api.datamodel_with_provider(
        r#"
            model cat {
                id      Int @id
                name    String
                externalTablesB ExternalTableB[]

                externalTableA ExternalTableA? @relation(fields: [externalTableId], references: [id])
                externalTableId Int?
            }

            model ExternalTableA {
                id      Int @id
                name    String
                cats    cat[]
            }

            model ExternalTableB {
                id      Int @id
                name    String
                cat     cat? @relation(fields: [catId], references: [id])
                catId   Int?
            }
        "#,
    );

    let filter = SchemaFilter {
        external_tables: vec!["ExternalTableA".to_string(), "ExternalTableB".to_string()],
    };
    api.create_migration_with_filter("modify", &schema_2, &dir, filter, "")
        .send_sync()
        .assert_migration_directories_count(2)
        .assert_migration("modify", move |migration| {
            // migration contains only add foreign key statements on the non external table
            let expected_script = if is_postgres {
                expect![[r#"
                    -- AlterTable
                    ALTER TABLE "cat" ADD COLUMN     "externalTableId" INTEGER;

                    -- AddForeignKey
                    ALTER TABLE "cat" ADD CONSTRAINT "cat_externalTableId_fkey" FOREIGN KEY ("externalTableId") REFERENCES "ExternalTableA"("id") ON DELETE SET NULL ON UPDATE CASCADE;
                "#]]
            } else if is_mysql {
                expect![[r#"
                    -- AlterTable
                    ALTER TABLE `cat` ADD COLUMN `externalTableId` INTEGER NULL;

                    -- AddForeignKey
                    ALTER TABLE `cat` ADD CONSTRAINT `cat_externalTableId_fkey` FOREIGN KEY (`externalTableId`) REFERENCES `ExternalTableA`(`id`) ON DELETE SET NULL ON UPDATE CASCADE;
                "#]]
            } else if is_sqlite {
                expect![[r#"
                    -- RedefineTables
                    PRAGMA defer_foreign_keys=ON;
                    PRAGMA foreign_keys=OFF;
                    CREATE TABLE "new_cat" (
                        "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                        "name" TEXT NOT NULL,
                        "externalTableId" INTEGER,
                        CONSTRAINT "cat_externalTableId_fkey" FOREIGN KEY ("externalTableId") REFERENCES "ExternalTableA" ("id") ON DELETE SET NULL ON UPDATE CASCADE
                    );
                    INSERT INTO "new_cat" ("id", "name") SELECT "id", "name" FROM "cat";
                    DROP TABLE "cat";
                    ALTER TABLE "new_cat" RENAME TO "cat";
                    PRAGMA foreign_keys=ON;
                    PRAGMA defer_foreign_keys=OFF;
                "#]]
            } else if is_mssql {
                expect![[r#"
                    BEGIN TRY

                    BEGIN TRAN;

                    -- AlterTable
                    ALTER TABLE [dbo].[cat] ADD [externalTableId] INT;

                    -- AddForeignKey
                    ALTER TABLE [dbo].[cat] ADD CONSTRAINT [cat_externalTableId_fkey] FOREIGN KEY ([externalTableId]) REFERENCES [dbo].[ExternalTableA]([id]) ON DELETE SET NULL ON UPDATE CASCADE;

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

#[test_connector(exclude(CockroachDb, Vitess))]
fn schema_filter_leveraging_init_script(api: TestApi) {
    // Creating the external table through the init script so it exists in the shadow db.
    // Therefore it can be referenced with a foreign key constraint from the Cat model without being created by a Prisma migration itself.
    let init_script = if api.is_mssql() {
        r#"CREATE TABLE [external] (id INT);"#
    } else {
        r#"CREATE TABLE external (id INT);"#
    };

    let schema = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
            externalTable external? @relation(fields: [externalTableId], references: [id])
            externalTableId Int?
        }

        model external {
            id      Int @id
            cats    Cat[]
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    let is_postgres = api.is_postgres();
    let is_mysql = api.is_mysql();
    let is_sqlite = api.is_sqlite();
    let is_mssql = api.is_mssql();

    let filter = SchemaFilter {
        external_tables: vec!["external".to_string()],
    };
    api.create_migration_with_filter("custom", &schema, &dir, filter, init_script)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("custom", move |migration| {
            // migration contains only add foreign key statements on the non external table
            let expected_script = if is_postgres {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE "Cat" (
                        "id" INTEGER NOT NULL,
                        "name" TEXT NOT NULL,
                        "externalTableId" INTEGER,

                        CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                    );

                    -- AddForeignKey
                    ALTER TABLE "Cat" ADD CONSTRAINT "Cat_externalTableId_fkey" FOREIGN KEY ("externalTableId") REFERENCES "external"("id") ON DELETE SET NULL ON UPDATE CASCADE;
                "#]]
            } else if is_mysql {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE `Cat` (
                        `id` INTEGER NOT NULL,
                        `name` VARCHAR(191) NOT NULL,
                        `externalTableId` INTEGER NULL,

                        PRIMARY KEY (`id`)
                    ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

                    -- AddForeignKey
                    ALTER TABLE `Cat` ADD CONSTRAINT `Cat_externalTableId_fkey` FOREIGN KEY (`externalTableId`) REFERENCES `external`(`id`) ON DELETE SET NULL ON UPDATE CASCADE;
                "#]]
            } else if is_sqlite {
                expect![[r#"
                    -- CreateTable
                    CREATE TABLE "Cat" (
                        "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                        "name" TEXT NOT NULL,
                        "externalTableId" INTEGER,
                        CONSTRAINT "Cat_externalTableId_fkey" FOREIGN KEY ("externalTableId") REFERENCES "external" ("id") ON DELETE SET NULL ON UPDATE CASCADE
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
                        [externalTableId] INT,
                        CONSTRAINT [Cat_pkey] PRIMARY KEY CLUSTERED ([id])
                    );

                    -- AddForeignKey
                    ALTER TABLE [dbo].[Cat] ADD CONSTRAINT [Cat_externalTableId_fkey] FOREIGN KEY ([externalTableId]) REFERENCES [dbo].[external]([id]) ON DELETE SET NULL ON UPDATE CASCADE;

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

#[test_connector(tags(Postgres, Mssql), exclude(CockroachDb))]
fn schema_filter_migration_multi_schema_requires_namespaced_table_names(api: TestApi) {
    let schema = api.datamodel_with_provider_and_features(
        r#"
        model Cat {
            id      Int @id
            name    String

            @@schema("one")
        }

        model ExternalTable {
            id      Int @id
            name    String

            @@schema("two")
        }
    "#,
        &[("schemas", "[\"one\", \"two\"]")],
        &[],
    );

    let dir = api.create_migrations_directory();

    let is_postgres = api.is_postgres();
    let is_mssql = api.is_mssql();

    let filter = SchemaFilter {
        external_tables: vec!["two.ExternalTable".to_string()],
    };
    api.create_migration_with_filter("custom", &schema, &dir, filter, "")
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("custom", move |migration| {
            // migration contains only non external table
            let expected_script = if is_postgres {
                expect![[r#"
                -- CreateSchema
                CREATE SCHEMA IF NOT EXISTS "one";

                -- CreateTable
                CREATE TABLE "one"."Cat" (
                    "id" INTEGER NOT NULL,
                    "name" TEXT NOT NULL,

                    CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                );
            "#]]
            } else if is_mssql {
                expect![[r#"
                    BEGIN TRY

                    BEGIN TRAN;

                    -- CreateSchema
                    EXEC sp_executesql N'CREATE SCHEMA [one];';;

                    -- CreateTable
                    CREATE TABLE [one].[Cat] (
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

#[test_connector(tags(Postgres, Mssql), exclude(CockroachDb))]
fn schema_filter_migration_multi_schema_without_namespaced_table_names(api: TestApi) {
    let schema = api.datamodel_with_provider_and_features(
        r#"
        model Cat {
            id      Int @id

            @@schema("one")
        }

        model ExternalTable {
            id      Int @id

            @@schema("two")
        }
    "#,
        &[("schemas", "[\"one\", \"two\"]")],
        &[],
    );

    let dir = api.create_migrations_directory();

    let filter = SchemaFilter {
        external_tables: vec!["ExternalTable".to_string()],
    };
    let err = api
        .create_migration_with_filter("custom", &schema, &dir, filter, "")
        .send_unwrap_err();

    assert_eq!(err.error_code(), Some("P3023"));
    assert_eq!(
        err.message(),
        Some(
            "When using an explicit schemas list in your datasource, `externalTables` in your prisma config must contain only fully qualified table names (e.g. `schema_name.table_name`)."
        )
    );
}

#[test_connector(exclude(CockroachDb))]
fn schema_filter_migration_with_namespaced_table_names_and_no_explicit_schemas_list(api: TestApi) {
    let schema = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
        }

        model ExternalTable {
            id      Int @id
        }
    "#,
    );

    let dir = api.create_migrations_directory();

    let filter = SchemaFilter {
        external_tables: vec!["public.ExternalTable".to_string()],
    };
    let err = api
        .create_migration_with_filter("custom", &schema, &dir, filter, "")
        .send_unwrap_err();

    assert_eq!(err.error_code(), Some("P3024"));
    assert_eq!(
        err.message(),
        Some(
            "When using no explicit schemas list in your datasource, `externalTables` in your prisma config must contain only simple table names without a schema name."
        )
    );
}

#[test_connector]
fn schema_filter_migration_dev_diagnostic_drift_detection(api: TestApi) {
    api.raw_cmd("CREATE TABLE external_table (id INTEGER NOT NULL, name TEXT NOT NULL, PRIMARY KEY (id));");

    let dir = api.create_migrations_directory();

    let filter = SchemaFilter {
        external_tables: vec!["external_table".to_string()],
    };
    // Table exists in DB and is missing in the schema but is marked as external => not a drift.
    api.dev_diagnostic_with_filter(&dir, filter)
        .send()
        .assert_is_create_migration();
}
