mod mssql;
mod multi_file;
mod mysql;
mod postgresql;
mod relation_mode;
mod sqlite;
mod vitess;

use barrel::types;
use indoc::{formatdoc, indoc};
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;

#[test_connector(exclude(CockroachDb))]
async fn mapped_model_name(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("_User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("_User_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Custom_User {
            id               Int         @id @default(autoincrement())

            @@map("_User")
        }
    "#};

    let expected = expect![[r#"
        model Custom_User {
          id Int @id @default(autoincrement())

          @@map("_User")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These models were enriched with `@@map` information taken from the previous Prisma schema:
          - "Custom_User"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn manually_overwritten_mapped_field_name(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("_test", types::integer());

                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model User {
            id               Int         @id @default(autoincrement())
            custom_test      Int         @map("_test")
        }
    "#};

    let expected = expect![[r#"
        model User {
          id          Int @id @default(autoincrement())
          custom_test Int @map("_test")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expected).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These fields were enriched with `@map` information taken from the previous Prisma schema:
          - Model: "User", field: "custom_test"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expectation).await;

    Ok(())
}

#[test_connector(exclude(Mssql, Mysql, CockroachDb))]
async fn mapped_model_and_field_name(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Post {
            id               Int         @id @default(autoincrement())
            c_user_id        Int         @map("user_id")
            Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
        }

        model Custom_User {
            c_id             Int         @id @default(autoincrement()) @map("id")
            Post             Post[]

            @@map(name: "User")
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id          Int         @id @default(autoincrement())
          c_user_id   Int         @map("user_id")
          Custom_User Custom_User @relation(fields: [c_user_id], references: [c_id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Custom_User {
          c_id Int    @id @default(autoincrement()) @map("id")
          Post Post[]

          @@map("User")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    let expected = expect![[r#"
        *** WARNING ***

        These fields were enriched with `@map` information taken from the previous Prisma schema:
          - Model: "Post", field: "c_user_id"
          - Model: "Custom_User", field: "c_id"

        These models were enriched with `@@map` information taken from the previous Prisma schema:
          - "Custom_User"
    "#]];

    expected.assert_eq(&api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn manually_mapped_model_and_field_name(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("_User", |t| {
                t.add_column("_id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_foreign_key(&["user_id"], "_User", &["_id"]);
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Post {
            id               Int         @id @default(autoincrement())
            c_user_id        Int         @map("user_id")
            Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
        }

        model Custom_User {
            c_id             Int         @id @default(autoincrement()) @map("_id")
            Post             Post[]

            @@map(name: "_User")
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id          Int         @id @default(autoincrement())
          c_user_id   Int         @map("user_id")
          Custom_User Custom_User @relation(fields: [c_user_id], references: [c_id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Custom_User {
          c_id Int    @id @default(autoincrement()) @map("_id")
          Post Post[]

          @@map("_User")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    let expected = expect![[r#"
        *** WARNING ***

        These fields were enriched with `@map` information taken from the previous Prisma schema:
          - Model: "Post", field: "c_user_id"
          - Model: "Custom_User", field: "c_id"

        These models were enriched with `@@map` information taken from the previous Prisma schema:
          - "Custom_User"
    "#]];

    expected.assert_eq(&api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn mapped_field_name(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id_1", types::integer());
                t.add_column("id_2", types::integer());
                t.add_column("index", types::integer());
                t.add_column("unique_1", types::integer());
                t.add_column("unique_2", types::integer());

                t.add_constraint(
                    "sqlite_autoindex_User_1",
                    types::unique_constraint(vec!["unique_1", "unique_2"]),
                );

                t.add_index("test2", types::index(vec!["index"]));

                t.add_constraint("User_pkey", types::primary_constraint(["id_1", "id_2"]));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model User {
            c_id_1      Int     @map("id_1")
            id_2        Int
            c_index     Int     @map("index")
            c_unique_1  Int     @map("unique_1")
            unique_2    Int

            @@id([c_id_1, id_2])
            @@index([c_index], map: "test2")
            @@unique([c_unique_1, unique_2], map: "sqlite_autoindex_User_1")
        }
    "#};

    let expected = expect![[r#"
        model User {
          c_id_1     Int @map("id_1")
          id_2       Int
          c_index    Int @map("index")
          c_unique_1 Int @map("unique_1")
          unique_2   Int

          @@id([c_id_1, id_2])
          @@unique([c_unique_1, unique_2], map: "sqlite_autoindex_User_1")
          @@index([c_index], map: "test2")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These fields were enriched with `@map` information taken from the previous Prisma schema:
          - Model: "User", field: "c_id_1"
          - Model: "User", field: "c_index"
          - Model: "User", field: "c_unique_1"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(capabilities(Enums), exclude(CockroachDb))]
async fn mapped_enum_name(api: &mut TestApi) -> TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.raw_cmd("CREATE TYPE color AS ENUM ( \'black\', \'white\')").await;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());

                let typ = if sql_family.is_postgres() {
                    "color"
                } else {
                    "ENUM ('black', 'white')"
                };

                t.add_column("color", types::custom(typ).nullable(false));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let enum_name = if sql_family.is_postgres() {
        "color"
    } else {
        "User_color"
    };

    let input_dm = format!(
        r#"
        model User {{
            id               Int @id @default(autoincrement())
            color            BlackNWhite
        }}

        enum BlackNWhite {{
            black
            white

            @@map("{enum_name}")
        }}
    "#
    );

    let final_dm = format!(
        r#"
        model User {{
            id               Int @id @default(autoincrement())
            color            BlackNWhite
        }}

        model Unrelated {{
            id               Int @id @default(autoincrement())
        }}

        enum BlackNWhite {{
            black
            white

            @@map("{enum_name}")
        }}
    "#
    );

    let result = api.re_introspect(&input_dm).await?;
    api.assert_eq_datamodels(&final_dm, &result);

    let expected = expect![[r#"
        *** WARNING ***

        These enums were enriched with `@@map` information taken from the previous Prisma schema:
          - "BlackNWhite"
    "#]];

    api.expect_re_introspect_warnings(&input_dm, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn manually_remapped_enum_value_name(api: &mut TestApi) -> TestResult {
    let sql = "CREATE Type color as ENUM (\'_black\', \'white\')";
    api.database().execute_raw(sql, &[]).await?;

    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("color", types::custom("color").nullable(false).default("_black"));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model User {
            id               Int @id @default(autoincrement())
            color            color @default(BLACK)
        }

        enum color {
            BLACK @map("_black")
            white
        }
    "#
    };

    let final_dm = expect![[r#"
        model User {
          id    Int   @id @default(autoincrement())
          color color @default(BLACK)
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }

        enum color {
          BLACK @map("_black")
          white
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, final_dm).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These enum values were enriched with `@map` information taken from the previous Prisma schema:
          - Enum: "color", value: "BLACK"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn manually_re_mapped_enum_name(api: &mut TestApi) -> TestResult {
    let sql = "CREATE Type _color as ENUM (\'black\', \'white\')";
    api.database().execute_raw(sql, &[]).await?;

    api.barrel()
        .execute(|migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("color", types::custom("_color").nullable(false));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model User {
            id               Int @id @default(autoincrement())
            color            BlackNWhite
        }

        enum BlackNWhite{
            black
            white

            @@map("_color")
        }
    "#};

    let final_dm = expect![[r#"
        model User {
          id    Int         @id @default(autoincrement())
          color BlackNWhite
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }

        enum BlackNWhite {
          black
          white

          @@map("_color")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, final_dm).await;

    let expected = expect![[r#"
        *** WARNING ***

        These enums were enriched with `@@map` information taken from the previous Prisma schema:
          - "BlackNWhite"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn manually_re_mapped_invalid_enum_values(api: &mut TestApi) -> TestResult {
    api.raw_cmd(r#"CREATE TYPE "invalid" as ENUM ('@', '-')"#).await;

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("sign", types::custom("invalid").nullable(false));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = r#"
        model User {
            id               Int @id @default(autoincrement())
            sign             invalid
        }

        enum invalid {
            at      @map("@")
            dash    @map("-")
        }
    "#;

    let final_dm = expect![[r#"
        model User {
          id   Int     @id @default(autoincrement())
          sign invalid
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }

        enum invalid {
          at   @map("@")
          dash @map("-")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, final_dm).await;

    let expected = expect![[r#"
        *** WARNING ***

        These enum values were enriched with `@map` information taken from the previous Prisma schema:
          - Enum: "invalid", value: "at"
          - Enum: "invalid", value: "dash"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql, CockroachDb, Sqlite))]
async fn multiple_changed_relation_names(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Employee", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Schedule", |t| {
                t.add_column("id", types::primary());
                t.add_column("morningEmployeeId", types::integer().nullable(false));
                t.add_column("eveningEmployeeId", types::integer().nullable(false));

                t.add_foreign_key(&["morningEmployeeId"], "Employee", &["id"]);
                t.add_foreign_key(&["eveningEmployeeId"], "Employee", &["id"]);
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Employee {
            id                                            Int         @id @default(autoincrement())
            A                                             Schedule[]  @relation("EmployeeToSchedule_eveningEmployeeId")
            Schedule_EmployeeToSchedule_morningEmployeeId Schedule[]  @relation("EmployeeToSchedule_morningEmployeeId")
        }

        model Schedule {
            id                                            Int         @id @default(autoincrement())
            morningEmployeeId                             Int
            eveningEmployeeId                             Int
            Employee_EmployeeToSchedule_eveningEmployeeId Employee    @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            Employee_EmployeeToSchedule_morningEmployeeId Employee    @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }
    "#};

    let expected = expect![[r#"
        model Employee {
          id                                            Int        @id @default(autoincrement())
          A                                             Schedule[] @relation("EmployeeToSchedule_eveningEmployeeId")
          Schedule_EmployeeToSchedule_morningEmployeeId Schedule[] @relation("EmployeeToSchedule_morningEmployeeId")
        }

        model Schedule {
          id                                            Int      @id @default(autoincrement())
          morningEmployeeId                             Int
          eveningEmployeeId                             Int
          Employee_EmployeeToSchedule_eveningEmployeeId Employee @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction)
          Employee_EmployeeToSchedule_morningEmployeeId Employee @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn custom_virtual_relation_field_names(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false).unique(true));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Post {
            id               Int @id @default(autoincrement())
            user_id          Int  @unique
            custom_User      User @relation(fields: [user_id], references: [id])
        }

        model User {
            id               Int @id @default(autoincrement())
            custom_Post      Post?
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id          Int  @id @default(autoincrement())
          user_id     Int  @unique
          custom_User User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id          Int   @id @default(autoincrement())
          custom_Post Post?
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn custom_model_order(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("A", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("A_pkey", types::primary_constraint(vec!["id"]));
            });
            migration.create_table("B", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("B_pkey", types::primary_constraint(vec!["id"]));
            });
            migration.create_table("J", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("J_pkey", types::primary_constraint(vec!["id"]));
            });
            migration.create_table("F", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("F_pkey", types::primary_constraint(vec!["id"]));
            });
            migration.create_table("Z", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Z_pkey", types::primary_constraint(vec!["id"]));
            });
            migration.create_table("M", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("M_pkey", types::primary_constraint(vec!["id"]));
            });
            migration.create_table("L", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("L_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model B {
            id               Int @id @default(autoincrement())
        }

        model A {
            id               Int @id @default(autoincrement())
        }

        model F {
            id               Int @id @default(autoincrement())
        }

        model C {
            id               Int @id @default(autoincrement())
        }

        model J {
            id               Int @id @default(autoincrement())
        }

        model Z {
            id               Int @id @default(autoincrement())
        }

        model K {
            id               Int @id @default(autoincrement())
        }
    "#};

    let final_dm = indoc! {r#"
        model B {
            id               Int @id @default(autoincrement())
        }

        model A {
            id               Int @id @default(autoincrement())
        }

        model F {
            id               Int @id @default(autoincrement())
        }

        model J {
            id               Int @id @default(autoincrement())
        }

        model Z {
            id               Int @id @default(autoincrement())
        }

        model L {
            id               Int @id @default(autoincrement())
        }

        model M {
            id               Int @id @default(autoincrement())
        }
    "#};

    let result = api.re_introspect(input_dm).await?;
    api.assert_eq_datamodels(final_dm, &result);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn custom_enum_order(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TYPE a AS ENUM ('id');
        CREATE TYPE b AS ENUM ('id');
        CREATE TYPE j AS ENUM ('id');
        CREATE TYPE f AS ENUM ('id');
        CREATE TYPE z AS ENUM ('id');
        CREATE TYPE m AS ENUM ('id');
        CREATE TYPE l AS ENUM ('id');
    "#;
    api.raw_cmd(sql).await;

    let input_dm = indoc! {r#"
        enum b {
            id
        }

        enum a {
            id
        }

        enum f {
            id
        }

        enum c {
            id
        }

        enum j {
            id
        }

        enum z {
            id
        }

        enum k {
            id
        }
    "#};

    let final_dm = indoc! {r#"
        enum b {
            id
        }

        enum a {
            id
        }

        enum f {
            id
        }

        enum j {
            id
        }

        enum z {
            id
        }

        enum l {
            id
        }

        enum m {
            id
        }
    "#};

    let result = api.re_introspect(input_dm).await?;
    api.assert_eq_datamodels(final_dm, &result);

    Ok(())
}

#[test_connector(exclude(Mssql, Mysql, Sqlite, CockroachDb))]
async fn multiple_changed_relation_names_due_to_mapped_models(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false).unique(true));
                t.add_column("user_id2", types::integer().nullable(false).unique(true));

                t.add_foreign_key(&["user_id"], "User", &["id"]);
                t.add_foreign_key(&["user_id2"], "User", &["id"]);
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Post {
            id               Int @id @default(autoincrement())
            user_id          Int  @unique
            user_id2         Int  @unique
            custom_User      Custom_User @relation("CustomRelationName", fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
            custom_User2     Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Custom_User {
            id               Int @id @default(autoincrement())
            custom_Post      Post? @relation("CustomRelationName")
            custom_Post2     Post? @relation("AnotherCustomRelationName")

            @@map("User")
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id           Int         @id @default(autoincrement())
          user_id      Int         @unique
          user_id2     Int         @unique
          custom_User2 Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id], onDelete: NoAction, onUpdate: NoAction)
          custom_User  Custom_User @relation("CustomRelationName", fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Custom_User {
          id           Int   @id @default(autoincrement())
          custom_Post2 Post? @relation("AnotherCustomRelationName")
          custom_Post  Post? @relation("CustomRelationName")

          @@map("User")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn virtual_cuid_default(api: &mut TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::varchar(30).primary(true));
                t.add_column("non_id", types::varchar(30));
            });

            migration.create_table("User2", |t| {
                t.add_column("id", types::varchar(36).primary(true));
            });

            migration.create_table("User3", |t| {
                t.add_column("id", types::varchar(21).primary(true));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await
        .unwrap();

    let input_dm = r#"
        model User {
            id        String    @id @default(cuid()) @db.VarChar(30)
            non_id    String    @default(cuid()) @db.VarChar(30)
        }

        model User2 {
            id        String    @id @default(uuid()) @db.VarChar(36)
        }

        model User3 {
            id        String    @id @default(nanoid(7)) @db.VarChar(21)
        }
        "#;

    let final_dm = indoc! {r#"
        model User {
            id        String    @id @default(cuid()) @db.VarChar(30)
            non_id    String    @default(cuid()) @db.VarChar(30)
        }

        model User2 {
            id        String    @id @default(uuid()) @db.VarChar(36)
        }

        model User3 {
            id        String    @id @default(nanoid(7)) @db.VarChar(21)
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }
    "#};

    let result = api.re_introspect(input_dm).await.unwrap();
    api.assert_eq_datamodels(final_dm, &result);
}

#[test_connector(tags(CockroachDb))]
async fn virtual_cuid_default_cockroach(api: &mut TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::varchar(30).primary(true));
                t.add_column("non_id", types::varchar(30));
            });

            migration.create_table("User2", |t| {
                t.add_column("id", types::varchar(36).primary(true));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await
        .unwrap();

    let input_dm = r#"
        model User {
            id        String    @id @default(cuid()) @db.String(30)
            non_id    String    @default(cuid()) @db.String(30)
        }

        model User2 {
            id        String    @id @default(uuid()) @db.String(36)
        }
        "#;

    let final_dm = indoc! {r#"
        model User {
            id        String    @id @default(cuid()) @db.String(30)
            non_id    String    @default(cuid()) @db.String(30)
        }

        model User2 {
            id        String    @id @default(uuid()) @db.String(36)
        }

        model Unrelated {
            id               BigInt @id @default(autoincrement())
        }
    "#};

    let result = api.re_introspect(input_dm).await.unwrap();
    api.assert_eq_datamodels(final_dm, &result);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn comments_should_be_kept(api: &mut TestApi) -> TestResult {
    api.raw_cmd("CREATE TYPE a AS ENUM (\'A\')").await;

    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("User2", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        /// A really helpful comment about the model
        model User {
            /// A really helpful comment about the field
            id         Int @id @default(autoincrement())
        }

        model User2 {
            id         Int @id @default(autoincrement())
        }

        /// A really helpful comment about the enum
        enum a {
            A // A really helpful comment about enum variant
        }

        /// just floating around here
    "#};

    let final_dm = expect![[r#"
        /// A really helpful comment about the model
        model User {
          /// A really helpful comment about the field
          id Int @id @default(autoincrement())
        }

        model User2 {
          id Int @id @default(autoincrement())
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }

        /// A really helpful comment about the enum
        enum a {
          A
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, final_dm).await;

    Ok(())
}

#[test_connector(exclude(Mssql, CockroachDb))]
async fn updated_at(api: &mut TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("lastupdated", types::datetime().nullable(true));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await
        .unwrap();

    let native_datetime = if api.sql_family().is_postgres() {
        "@db.Timestamp(6)"
    } else if api.sql_family().is_mysql() {
        "@db.DateTime(0)"
    } else {
        ""
    };
    let input_dm = formatdoc! {r#"
        model User {{
            id           Int @id @default(autoincrement())
            lastupdated  DateTime?  @updatedAt {native_datetime}
        }}
        "#,
    };

    let final_dm = formatdoc! {r#"
        model User {{
            id           Int @id @default(autoincrement())
            lastupdated  DateTime?  @updatedAt {native_datetime}
        }}

        model Unrelated {{
            id               Int @id @default(autoincrement())
        }}
        "#
    };

    let result = api.re_introspect(&input_dm).await.unwrap();
    api.assert_eq_datamodels(&final_dm, &result);
}

#[test_connector(exclude(Vitess, CockroachDb))]
async fn multiple_many_to_many_on_same_model(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("A", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("A_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("B", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("B_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("_AToB", |t| {
                t.add_column("A", types::integer().nullable(false));
                t.add_column("B", types::integer().nullable(false));

                t.add_index("test2", types::index(vec!["A", "B"]).unique(true));
                t.add_index("test4", types::index(vec!["B"]));

                t.add_foreign_key(&["A"], "A", &["id"]);
                t.add_foreign_key(&["B"], "B", &["id"]);
            });

            migration.create_table("_AToB2", |t| {
                t.add_column("A", types::integer().nullable(false));
                t.add_column("B", types::integer().nullable(false));

                t.add_index("test", types::index(vec!["A", "B"]).unique(true));
                t.add_index("test3", types::index(vec!["B"]));

                t.add_foreign_key(&["A"], "A", &["id"]);
                t.add_foreign_key(&["B"], "B", &["id"]);
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model B {
            id              Int @id @default(autoincrement())
            custom_A        A[]
            special_A       A[] @relation("AToB2")
        }

        model A {
            id              Int @id @default(autoincrement())
            custom_B        B[]
            special_B       B[] @relation("AToB2")
        }
    "#};

    let final_dm = expect![[r#"
        model B {
          id        Int @id @default(autoincrement())
          custom_A  A[] @relation("AToB")
          special_A A[] @relation("AToB2")
        }

        model A {
          id        Int @id @default(autoincrement())
          custom_B  B[] @relation("AToB")
          special_B B[] @relation("AToB2")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, final_dm).await;

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn re_introspecting_mysql_enum_names(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE `User` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY,
            color  ENUM('black', 'white') NOT NULL
        );

        CREATE TABLE `Unrelated` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY
        );
    "#;
    api.raw_cmd(sql).await;

    let input_dm = r#"
            model User {
               id               Int @id @default(autoincrement())
               color            BlackNWhite
            }

            enum BlackNWhite{
                black
                white
            }
        "#;

    let expected = expect![[r#"
        model User {
          id    Int         @id @default(autoincrement())
          color BlackNWhite
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }

        enum BlackNWhite {
          black
          white
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expected).await;

    let expected = expect![[r#""#]];
    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn re_introspecting_mysql_enum_names_if_enum_is_reused(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE `User` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY,
            color  ENUM('black', 'white') NOT NULL,
            color2 ENUM('black', 'white') NOT NULL
        );

        CREATE TABLE `Unrelated` (
            id INTEGER AUTO_INCREMENT PRIMARY KEY
        );
    "#;
    api.raw_cmd(sql).await;

    let input_dm = r#"
            model User {
               id               Int @id @default(autoincrement())
               color            BlackNWhite
               color2           BlackNWhite
            }

            enum BlackNWhite{
                black
                white
            }
        "#;

    let expected = expect![[r#"
        model User {
          id     Int         @id @default(autoincrement())
          color  BlackNWhite
          color2 BlackNWhite
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }

        enum BlackNWhite {
          black
          white
        }
    "#]];
    api.expect_re_introspected_datamodel(input_dm, expected).await;
    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn custom_repro(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "tag" (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE "Post" (
            id SERIAL PRIMARY KEY,
            tag_id INTEGER NOT NULL REFERENCES tag(id)
        );

        CREATE TABLE "Unrelated" (
            id SERIAL PRIMARY KEY
        );
    "#;
    api.raw_cmd(sql).await;

    let input_dm = indoc! {r#"
        model Post{
          id        Int       @id @default(autoincrement())
          tag_id    Int
          tag       Tag       @relation("post_to_tag", fields:[tag_id], references: id)
        }

        model Tag {
          id        Int       @id @default(autoincrement())
          name      String    @unique
          posts     Post[]    @relation("post_to_tag")
          @@map("tag")
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id     Int @id @default(autoincrement())
          tag_id Int
          tag    Tag @relation("post_to_tag", fields: [tag_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Tag {
          id    Int    @id @default(autoincrement())
          name  String @unique
          posts Post[] @relation("post_to_tag")

          @@map("tag")
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn re_introspecting_ignore(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("test", types::integer().nullable(true));

                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Ignored", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("test", types::integer().nullable(true));

                t.add_constraint("Ignored_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model User {
            id           Int @id @default(autoincrement())
            test         Int?      @ignore
        }

        model Ignored {
            id           Int @id @default(autoincrement())
            test         Int?

            @@ignore
        }
    "#};

    let final_dm = indoc! {r#"
        model User {
            id           Int @id @default(autoincrement())
            test         Int?      @ignore
        }

        model Ignored {
            id           Int @id @default(autoincrement())
            test         Int?

            @@ignore
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }
    "#};

    let result = api.re_introspect(input_dm).await.unwrap();
    api.assert_eq_datamodels(final_dm, &result);

    Ok(())
}

#[test_connector(exclude(Vitess, CockroachDb, Sqlite))]
async fn do_not_try_to_keep_custom_many_to_many_self_relation_names(api: &mut TestApi) -> TestResult {
    // We do not have enough information to correctly assign which field should point to column A in the
    // join table and which one to B
    // Upon table creation this is dependant on lexicographic order of the names of the fields, but we
    // cannot be sure that users keep the order the same when renaming. worst case would be we accidentally
    // switch the directions when reintrospecting.
    // The generated names are also not helpful though, but at least they don't give a false sense of correctness -.-
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("_FollowRelation", |t| {
                t.add_column("A", types::integer().nullable(false).unique(false));
                t.add_column("B", types::integer().nullable(false).unique(false));

                t.add_foreign_key(&["A"], "User", &["id"]);
                t.add_foreign_key(&["B"], "User", &["id"]);

                t.add_index("test", types::index(vec!["A", "B"]).unique(true));
                t.add_index("test2", types::index(vec!["B"]).unique(false));
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model User {
            id          Int       @id @default(autoincrement())
            followers   User[]    @relation("FollowRelation")
            following   User[]    @relation("FollowRelation")
        }
    "#};

    let final_dm = expect![[r#"
        model User {
          id     Int    @id @default(autoincrement())
          User_A User[] @relation("FollowRelation")
          User_B User[] @relation("FollowRelation")
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, final_dm).await;

    Ok(())
}

#[test_connector(tags(Postgres, Mssql, Mysql, Sqlite), exclude(CockroachDb))]
async fn re_introspecting_custom_compound_unique_upgrade(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
                t.add_column("first", types::integer());
                t.add_column("last", types::integer());
                t.add_index("compound", types::index(["first", "last"]).unique(true));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let input_dm = indoc! {r#"
         model User {
             id     Int @id @default(autoincrement())
             first  Int
             last   Int

             @@unique([first, last], name: "compound")
         }
     "#};

    let final_dm = indoc! {r#"
         model User {
             id     Int @id @default(autoincrement())
             first  Int
             last   Int

             @@unique([first, last], name: "compound", map: "compound")
         }

         model Unrelated {
             id    Int @id @default(autoincrement())
         }
     "#};

    let result = api.re_introspect(input_dm).await?;
    api.assert_eq_datamodels(final_dm, &result);

    Ok(())
}

#[test_connector(tags(Postgres12))]
async fn re_introspecting_custom_index_order(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table =
        format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, a jsonb not null, b jsonb not null, c jsonb not null)",);
    let create_idx_a = format!("CREATE INDEX \"aaaaaa\" ON \"{schema_name}\".\"A\" USING GIN (b);",);
    let create_idx_b = format!("CREATE INDEX \"bbbbbb\" ON \"{schema_name}\".\"A\" USING GIN (a);",);
    let create_idx_c = format!("CREATE INDEX \"cccccc\" ON \"{schema_name}\".\"A\" USING GIN (c);",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx_a).await?;
    api.database().raw_cmd(&create_idx_b).await?;
    api.database().raw_cmd(&create_idx_c).await?;

    let input_dm = indoc! {r#"
         model A {
           id Int   @id
           a  Json
           b  Json

           @@index([a], map: "bbbbbb", type: Gin)
           @@index([b], map: "aaaaaa", type: Gin)
         }
    "#};

    let re_introspected = api.re_introspect_dml(input_dm).await?;

    let expected = expect![[r#"
        model A {
          id Int  @id @default(autoincrement())
          a  Json
          b  Json
          c  Json

          @@index([a], map: "bbbbbb", type: Gin)
          @@index([b], map: "aaaaaa", type: Gin)
          @@index([c], map: "cccccc", type: Gin)
        }
    "#]];

    expected.assert_eq(&re_introspected);

    Ok(())
}

#[test_connector(tags(Postgres), preview_features("multiSchema"))]
async fn re_introspecting_with_schemas_property(api: &mut TestApi) -> TestResult {
    let create_schema = "CREATE SCHEMA \"first\"";
    let create_table = "CREATE TABLE \"first\".\"A\" (id TEXT PRIMARY KEY)";

    api.database().raw_cmd(create_schema).await?;
    api.database().raw_cmd(create_table).await?;

    let create_schema = "CREATE SCHEMA \"second\"";
    let create_table = "CREATE TABLE \"second\".\"B\" (id TEXT PRIMARY KEY)";

    api.database().raw_cmd(create_schema).await?;
    api.database().raw_cmd(create_table).await?;

    let input_dm = indoc! {r#"
          generator client {
           provider        = "prisma-client-js"
           previewFeatures = ["multiSchema"]
         }

         datasource myds {
           provider = "postgresql"
           url      = env("DATABASE_URL")
           schemas  = ["first", "second"]
         }
    "#};

    let re_introspected = api.re_introspect_config(input_dm).await?;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource myds {
          provider = "postgresql"
          url      = env("DATABASE_URL")
          schemas  = ["first", "second"]
        }

        model A {
          id String @id

          @@schema("first")
        }

        model B {
          id String @id

          @@schema("second")
        }
    "#]];

    expected.assert_eq(&re_introspected);

    Ok(())
}
