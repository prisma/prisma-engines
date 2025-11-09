use std::borrow::Cow;

use prisma_value::PrismaValue;
use sql_migration_tests::test_api::*;

const BASIC_ENUM_DM: &str = r#"
model Cat {
    id Int @id
    mood CatMood
}

enum CatMood {
    HAPPY
    HUNGRY
}
"#;

#[test_connector(capabilities(Enums))]
fn an_enum_can_be_turned_into_a_model(api: TestApi) {
    api.schema_push_w_datasource(BASIC_ENUM_DM).send().assert_green();

    let enum_name = if api.lower_cases_table_names() {
        "cat_mood"
    } else if api.is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    if api.is_sqlite() {
        api.assert_schema().assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_string())
        });
    } else {
        api.assert_schema().assert_enum(enum_name, |enm| enm);
    }

    let dm2 = r#"
        model Cat {
            id Int @id
            moodId Int
            mood CatMood @relation(fields: [moodId], references: [id])
        }

        model CatMood {
            id Int @id
            description String
            biteRisk Int
            c        Cat[]
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema()
        .assert_table("Cat", |table| table.assert_columns_count(2).assert_has_column("moodId"))
        .assert_table("CatMood", |table| table.assert_column_count(3))
        .assert_has_no_enum("CatMood");
}

#[test_connector(capabilities(Enums))]
fn variants_can_be_added_to_an_existing_enum(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let enum_name = if api.lower_cases_table_names() {
        "cat_mood"
    } else if api.is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    if api.is_sqlite() {
        api.assert_schema().assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_string())
        });
    } else {
        api.assert_schema()
            .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY"]));
    }

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
            HAPPY
            JOYJOY
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    if api.is_sqlite() {
        api.assert_schema().assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_string())
        });
    } else {
        api.assert_schema()
            .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY", "HAPPY", "JOYJOY"]));
    }
}

#[test_connector(capabilities(Enums))]
fn variants_can_be_removed_from_an_existing_enum(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let enum_name = if api.lower_cases_table_names() {
        "cat_mood"
    } else if api.is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    if api.is_sqlite() {
        api.assert_schema().assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_string())
        });
    } else {
        api.assert_schema()
            .assert_enum(enum_name, |enm| enm.assert_values(&["HAPPY", "HUNGRY"]));
    }

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
        }
    "#;

    let warnings: &[Cow<'_, str>] = if api.is_mysql() {
        &["The values [HAPPY] on the enum `Cat_mood` will be removed. If these variants are still used in the database, this will fail.".into()]
    } else if api.is_sqlite() {
        &[]
    } else {
        &["The values [HAPPY] on the enum `CatMood` will be removed. If these variants are still used in the database, this will fail.".into()]
    };

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_warnings(warnings)
        .assert_executable();

    if api.is_sqlite() {
        api.assert_schema().assert_table("Cat", |table| {
            table.assert_column("mood", |col| col.assert_type_is_string())
        });
    } else {
        api.assert_schema()
            .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY"]));
    }
}

#[test_connector(capabilities(Enums))]
fn models_with_enum_values_can_be_dropped(api: TestApi) {
    api.schema_push_w_datasource(BASIC_ENUM_DM).send().assert_green();

    api.assert_schema().assert_tables_count(1);

    api.insert("Cat").value("id", 1).value("mood", "HAPPY").result_raw();

    let warn = if api.lower_cases_table_names() {
        "You are about to drop the `cat` table, which is not empty (1 rows)."
    } else {
        "You are about to drop the `Cat` table, which is not empty (1 rows)."
    };

    api.schema_push_w_datasource("")
        .force(true)
        .send()
        .assert_executable()
        .assert_warnings(&[warn.into()]);

    api.assert_schema().assert_tables_count(0);
}

#[test_connector(capabilities(Enums))]
fn enum_field_to_string_field_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood?
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("mood", |col| {
            if api.is_sqlite() {
                col.assert_type_is_string()
            } else {
                col.assert_type_is_enum()
            }
        })
    });

    api.insert("Cat").value("id", 1).value("mood", "HAPPY").result_raw();

    let dm2 = r#"
        model Cat {
            id      Int @id
            mood    String?
        }
    "#;

    api.schema_push_w_datasource(dm2).force(true).send().assert_executable();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("mood", |col| col.assert_type_is_string())
    });
}

#[test_connector(capabilities(Enums))]
fn string_field_to_enum_field_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id      Int @id
            mood    String?
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("mood", |col| col.assert_type_is_string())
    });

    api.insert("Cat").value("id", 1).value("mood", "HAPPY").result_raw();

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood?
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    let warnings: &[Cow<'_, str>] = if api.is_postgres() {
        &["The `mood` column on the `Cat` table would be dropped and recreated. This will lead to data loss.".into()]
    } else if api.is_sqlite() {
        &[]
    } else if api.lower_cases_table_names() {
        &["You are about to alter the column `mood` on the `cat` table, which contains 1 non-null values. The data in that column will be cast from `VarChar(191)` to `Enum(EnumId(0))`.".into()]
    } else {
        &["You are about to alter the column `mood` on the `Cat` table, which contains 1 non-null values. The data in that column will be cast from `VarChar(191)` to `Enum(EnumId(0))`.".into()]
    };

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_executable()
        .assert_warnings(warnings);

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("mood", |col| {
            if api.is_sqlite() {
                col.assert_type_is_string()
            } else {
                col.assert_type_is_enum()
            }
        })
    });
}

#[test_connector(capabilities(Enums))]
fn enums_used_in_default_can_be_changed(api: TestApi) {
    let dm1 = r#"
        model Panther {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model Tiger {
            id Int @id
            mood CatMood @default(HAPPY)
        }

         model Leopard {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model Lion {
            id Int @id
            mood CatMood
        }

        model GoodDog {
            id Int @id
            mood DogMood @default(HAPPY)
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }

        enum DogMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_tables_count(5);

    let dm2 = r#"
        model Panther {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model Tiger {
            id Int @id
            mood CatMood @default(HAPPY)
        }

         model Leopard {
            id Int @id
            mood CatMood
        }

        model Lion {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        model GoodDog {
            id Int @id
            mood DogMood @default(HAPPY)
        }

        enum CatMood {
            HAPPY
            ANGRY
        }

        enum DogMood {
            HAPPY
            HUNGRY
            SLEEPY
        }
    "#;

    let warnings: &[Cow<'_, str>] = if api.is_postgres() {
        &["The values [HUNGRY] on the enum `CatMood` will be removed. If these variants are still used in the database, this will fail.".into()]
    } else if api.is_sqlite() {
        &[]
    } else {
        &["The values [HUNGRY] on the enum `Lion_mood` will be removed. If these variants are still used in the database, this will fail.".into(), "The values [HUNGRY] on the enum `Lion_mood` will be removed. If these variants are still used in the database, this will fail.".into()]
    };

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_executable()
        .assert_warnings(warnings);

    api.assert_schema().assert_tables_count(5);
}

#[test_connector(capabilities(Enums))]
fn changing_all_values_of_enums_used_in_defaults_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            morningMood             CatMood @default(HUNGRY)
            alternateMorningMood    CatMood @default(HUNGRY)
            afternoonMood           CatMood @default(HAPPY)
            eveningMood             CatMood @default(HUNGRY)
            defaultMood             CatMood
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model Cat {
            id Int @id
            morningMood             CatMood @default(MEOW)
            alternateMorningMood    CatMood @default(MEOWMEOWMEOW)
            afternoonMood           CatMood @default(PURR)
            eveningMood             CatMood @default(MEOWMEOW)
            defaultMood             CatMood
        }

        enum CatMood {
            MEOW
            MEOWMEOW
            MEOWMEOWMEOW
            PURR
        }
    "#;

    api.schema_push_w_datasource(dm2).force(true).send();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("eveningMood", |col| {
            if api.is_sqlite() {
                col.assert_default_value(&PrismaValue::String("MEOWMEOW".to_string()))
            } else {
                col.assert_enum_default("MEOWMEOW")
            }
        })
    });

    // Check that the migration is idempotent.
    api.schema_push_w_datasource(dm2).force(true).send().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn sqlite_text_is_picked_up_as_enum(api: TestApi) {
    let sql = r#"
        CREATE TABLE "Band" (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            genre TEXT NOT NULL
        );
    "#;

    api.raw_cmd(sql);

    let dm = r#"
        enum Genre {
            SKA
            PUNK
        }

        model Band {
            id Int @id @default(autoincrement())
            name String
            genre Genre
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Postgres))]
fn existing_enums_are_picked_up(api: TestApi) {
    let sql = r#"
        CREATE TYPE "Genre" AS ENUM ('SKA', 'PUNK');

        CREATE TABLE "public"."Band" (
            id BIGSERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            genre "Genre" NOT NULL
        );
    "#;

    api.raw_cmd(sql);

    let dm = r#"
        enum Genre {
            SKA
            PUNK
        }

        model Band {
            id BigInt @id @default(autoincrement())
            name String
            genre Genre
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

// Bug: https://github.com/prisma/prisma/issues/8137
#[test_connector(tags(Postgres))]
fn enum_array_modification_should_work(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm = r#"
        datasource test {
            provider = "postgres"
        }

        enum Position {
            First
            Second
            Last
        }

        model Test {
            id        String     @id @default(uuid())
            positions Position[]
        }
    "#;

    api.create_migration("01init", dm, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["01init"]);

    let dm = r#"
        datasource test {
            provider = "postgres"
        }

        enum Position {
            First
            Second
        }

        model Test {
            id        String     @id @default(uuid())
            positions Position[]
        }
    "#;

    api.create_migration("02remove", dm, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["02remove"]);

    api.create_migration("03empty", dm, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&[]);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn mapped_enum_defaults_must_work(api: TestApi) {
    let schema = r#"
        datasource db {
            provider = "postgres"
        }

        enum Color {
            Red @map("0")
            Green @map("Grün")
            Blue @map("Blu")
            Annoyed @map("pfuh 🙄...")
        }

        model Test {
            id Int @id
            mainColor Color @default(Green)
            secondaryColor Color @default(Red)
            colorOrdering Color[] @default([Blue, Red, Green, Red, Blue, Red])
        }
    "#;

    let expect = expect![[r#"
        -- CreateEnum
        CREATE TYPE "Color" AS ENUM ('0', 'Grün', 'Blu', 'pfuh 🙄...');

        -- CreateTable
        CREATE TABLE "Test" (
            "id" INTEGER NOT NULL,
            "mainColor" "Color" NOT NULL DEFAULT 'Grün',
            "secondaryColor" "Color" NOT NULL DEFAULT '0',
            "colorOrdering" "Color"[] DEFAULT ARRAY['Blu', '0', 'Grün', '0', 'Blu', '0']::"Color"[],

            CONSTRAINT "Test_pkey" PRIMARY KEY ("id")
        );
    "#]];
    api.expect_sql_for_schema(schema, &expect);

    api.schema_push(schema).send().assert_green();
    api.schema_push(schema).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn alter_enum_and_change_default_must_work(api: TestApi) {
    let plain_dm = r#"
        datasource db {
            provider = "postgres"
        }
        model Cat {
            id      Int    @id
            moods   Mood[] @default([])
        }
        enum Mood {
            SLEEPY
            MOODY
        }
    "#;

    api.schema_push(plain_dm).send().assert_green();

    let custom_dm = r#"
        datasource test {
            provider = "postgres"
        }
        model Cat {
            id      Int    @id
            moods   Mood[] @default([SLEEPY])
        }
        enum Mood {
            HUNGRY
            SLEEPY
        }
    "#;

    // recall: schema_push doesn't run if it has warnings. You need to specify "force(true)"
    api.schema_push(custom_dm).force(true).send().assert_warnings(&[Cow::from(
        "The values [MOODY] on the enum `Mood` will be removed. If these variants are still used in the database, this will fail.",
    )]);
    api.schema_push(custom_dm).send().assert_green().assert_no_steps();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("moods", |col| {
            col.assert_default_value(&PrismaValue::List(vec![PrismaValue::Enum("SLEEPY".to_string())]))
        })
    });

    // we repeat the same tests with migrations, so we can observe the generated SQL statements.
    api.reset().send_sync(None);
    api.assert_schema().assert_tables_count(0);

    let dir = api.create_migrations_directory();
    api.create_migration("plain", plain_dm, &dir).send_sync();

    api.create_migration("custom", custom_dm, &dir)
        .send_sync()
        .assert_migration_directories_count(2)
        .assert_migration("custom", move |migration| {
            let expected_script = expect![[r#"
                /*
                  Warnings:

                  - The values [MOODY] on the enum `Mood` will be removed. If these variants are still used in the database, this will fail.

                */
                -- AlterEnum
                BEGIN;
                CREATE TYPE "Mood_new" AS ENUM ('HUNGRY', 'SLEEPY');
                ALTER TABLE "public"."Cat" ALTER COLUMN "moods" DROP DEFAULT;
                ALTER TABLE "Cat" ALTER COLUMN "moods" TYPE "Mood_new"[] USING ("moods"::text::"Mood_new"[]);
                ALTER TYPE "Mood" RENAME TO "Mood_old";
                ALTER TYPE "Mood_new" RENAME TO "Mood";
                DROP TYPE "public"."Mood_old";
                ALTER TABLE "Cat" ALTER COLUMN "moods" SET DEFAULT ARRAY['SLEEPY']::"Mood"[];
                COMMIT;

                -- AlterTable
                ALTER TABLE "Cat" ALTER COLUMN "moods" SET DEFAULT ARRAY['SLEEPY']::"Mood"[];
            "#]];
            migration.expect_contents(expected_script)
        });
}
