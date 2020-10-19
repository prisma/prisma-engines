use barrel::types;
use indoc::indoc;
use introspection_engine_tests::{assert_eq_datamodels, assert_eq_json, test_api::*};
use quaint::prelude::Queryable;
use serde_json::json;
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector]
async fn mapped_model_name(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("_User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model Custom_User {
            id               Int         @id @default(autoincrement())

            @@map(name: "_User")
        }
    "#};

    let final_dm = indoc! {r#"
        model Custom_User {
            id               Int         @id @default(autoincrement())

            @@map(name: "_User")
        }

        model Unrelated {
            id               Int         @id @default(autoincrement())
        }
    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    let expected = json!([{
        "code": 7,
        "message": "These models were enriched with `@@map` information taken from the previous Prisma schema.",
        "affected": [{
            "model":"Custom_User"
        }]
    }]);

    assert_eq_json!(expected, api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_each_connector]
async fn manually_overwritten_mapped_field_name(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("_test", types::integer());
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model User {
            id               Int         @id @default(autoincrement())
            custom_test      Int         @map("_test")
        }
    "#};

    let final_dm = indoc! {r#"
        model User {
            id               Int         @id @default(autoincrement())
            custom_test      Int         @map("_test")
        }

        model Unrelated {
            id               Int         @id @default(autoincrement())
        }
    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    let expected = json!([{
        "code": 8,
        "message": "These fields were enriched with `@map` information taken from the previous Prisma schema.",
        "affected": [{
            "model": "User",
            "field": "custom_test"
        }]
    }]);

    assert_eq_json!(expected, api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_each_connector]
async fn mapped_model_and_field_name(api: &TestApi) -> crate::TestResult {
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

    let extra_index = if api.sql_family().is_mysql() {
        r#"@@index([c_user_id], name: "user_id")"#
    } else {
        ""
    };

    let input_dm = format!(
        r#"
        model Post {{
            id               Int         @id @default(autoincrement())
            c_user_id        Int         @map("user_id")
            Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
            {}
        }}

        model Custom_User {{
            c_id             Int         @id @default(autoincrement()) @map("id")
            Post             Post[]

            @@map(name: "User")
        }}
    "#,
        extra_index
    );

    let final_dm = format!(
        r#"
        model Post {{
            id               Int         @id @default(autoincrement())
            c_user_id        Int         @map("user_id")
            Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
            {}
        }}

        model Custom_User {{
            c_id             Int         @id @default(autoincrement()) @map("id")
            Post             Post[]

            @@map(name: "User")
        }}

        model Unrelated {{
            id               Int         @id @default(autoincrement())
        }}
    "#,
        extra_index
    );

    assert_eq_datamodels!(&final_dm, &api.re_introspect(&input_dm).await?);

    let expected = json!([
        {
            "code": 7,
            "message": "These models were enriched with `@@map` information taken from the previous Prisma schema.",
            "affected":[
                {
                    "model": "Custom_User"
                },
            ]
        },
        {
            "code": 8,
            "message": "These fields were enriched with `@map` information taken from the previous Prisma schema.",
            "affected": [
                {
                    "model": "Post",
                    "field": "c_user_id"
                },
                {
                    "model": "Custom_User",
                    "field": "c_id"
                }
            ]
        }
    ]);

    assert_eq_json!(expected, api.re_introspect_warnings(&input_dm).await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn manually_mapped_model_and_field_name(api: &TestApi) -> crate::TestResult {
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

    let extra_index = if api.sql_family().is_mysql() {
        r#"@@index([c_user_id], name: "user_id")"#
    } else {
        ""
    };

    let input_dm = format!(
        r#"
        model Post {{
            id               Int         @id @default(autoincrement())
            c_user_id        Int         @map("user_id")
            Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
            {}
        }}

        model Custom_User {{
            c_id             Int         @id @default(autoincrement()) @map("_id")
            Post             Post[]

            @@map(name: "_User")
        }}
    "#,
        extra_index
    );

    let final_dm = format!(
        r#"
        model Post {{
            id               Int         @id @default(autoincrement())
            c_user_id        Int         @map("user_id")
            Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
            {}
        }}

        model Custom_User {{
            c_id             Int         @id @default(autoincrement()) @map("_id")
            Post             Post[]

            @@map(name: "_User")
        }}

        model Unrelated {{
            id               Int         @id @default(autoincrement())
        }}
    "#,
        extra_index
    );

    assert_eq_datamodels!(&final_dm, &api.re_introspect(&input_dm).await?);

    let expected = json!([
        {
            "code": 7,
            "message": "These models were enriched with `@@map` information taken from the previous Prisma schema.",
            "affected": [{
                "model": "Custom_User"
            }]
        },
        {
            "code": 8,
            "message": "These fields were enriched with `@map` information taken from the previous Prisma schema.",
            "affected": [
                {
                    "model": "Post",
                    "field": "c_user_id"
                },
                {
                    "model": "Custom_User",
                    "field": "c_id"
                }
            ]
        }
    ]);

    assert_eq_json!(expected, api.re_introspect_warnings(&input_dm).await?);

    Ok(())
}

#[test_each_connector]
async fn mapped_field_name(api: &TestApi) -> crate::TestResult {
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

                t.set_primary_key(&["id_1", "id_2"]);
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
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
            @@index([c_index], name: "test2")
            @@unique([c_unique_1, unique_2], name: "sqlite_autoindex_User_1")
        }
    "#};

    let final_dm = indoc! {r#"
        model User {
            c_id_1      Int     @map("id_1")
            id_2        Int
            c_index     Int     @map("index")
            c_unique_1  Int     @map("unique_1")
            unique_2    Int

            @@id([c_id_1, id_2])
            @@index([c_index], name: "test2")
            @@unique([c_unique_1, unique_2], name: "sqlite_autoindex_User_1")
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }
    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    let expected = json!([{
        "code": 8,
        "message": "These fields were enriched with `@map` information taken from the previous Prisma schema.",
        "affected": [
            {
                "model": "User",
                "field": "c_id_1"
            },
            {
                "model": "User",
                "field": "c_index"
            },{
                "model": "User",
                "field": "c_unique_1"
            }
        ]
    }]);

    assert_eq_json!(expected, api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn mapped_enum_name(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        let sql = "CREATE Type color as ENUM ( \'black\', \'white\')";
        api.database().execute_raw(sql, &[]).await?;
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

            @@map("{}")
        }}
    "#,
        enum_name
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

            @@map("{}")
        }}
    "#,
        enum_name
    );

    assert_eq_datamodels!(&final_dm, &api.re_introspect(&input_dm).await?);

    let expected = json!([{
        "code": 9,
        "message": "These enums were enriched with `@@map` information taken from the previous Prisma schema.",
        "affected": [{
            "enm": "BlackNWhite"
        }]
    }]);

    assert_eq_json!(expected, api.re_introspect_warnings(&input_dm).await?);

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn mapped_enum_value_name(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        let sql = "CREATE Type color as ENUM (\'black\', \'white\')";
        api.database().execute_raw(sql, &[]).await?;
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

                t.add_column("color", types::custom(typ).nullable(false).default("black"));
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
            color            {0} @default(BLACK)
        }}

        enum {0} {{
            BLACK @map("black")
            white
        }}
    "#,
        enum_name
    );

    let final_dm = format!(
        r#"
        model User {{
            id               Int @id @default(autoincrement())
            color            {0} @default(BLACK)
        }}

        model Unrelated {{
            id               Int @id @default(autoincrement())
        }}

        enum {0} {{
            BLACK @map("black")
            white
        }}
    "#,
        enum_name
    );

    assert_eq_datamodels!(&final_dm, &api.re_introspect(&input_dm).await?);

    let expected = json!([{
        "code": 10,
        "message": "These enum values were enriched with `@map` information taken from the previous Prisma schema.",
        "affected" :[{
            "enm": enum_name,
            "value": "BLACK"
        }]
    }]);

    assert_eq_json!(expected, api.re_introspect_warnings(&input_dm).await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn manually_remapped_enum_value_name(api: &TestApi) -> crate::TestResult {
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

    let final_dm = indoc! {r#"
        model User {
            id               Int @id @default(autoincrement())
            color            color @default(BLACK)
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }

        enum color {
            BLACK @map("_black")
            white
        }
    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    let expected = json!([{
        "code": 10,
        "message": "These enum values were enriched with `@map` information taken from the previous Prisma schema.",
        "affected" :[{
            "enm": "color",
            "value": "BLACK"
        }]
    }]);

    assert_eq_json!(expected, api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn manually_re_mapped_enum_name(api: &TestApi) -> crate::TestResult {
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

    let final_dm = indoc! {r#"
        model User {
            id               Int @id @default(autoincrement())
            color            BlackNWhite
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }

        enum BlackNWhite{
            black
            white

            @@map("_color")
        }
    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    let expected = json!([{
        "code": 9,
        "message": "These enums were enriched with `@@map` information taken from the previous Prisma schema.",
        "affected": [{
            "enm": "BlackNWhite"
        }]
    }]);

    assert_eq_json!(expected, api.re_introspect_warnings(input_dm).await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn manually_re_mapped_invalid_enum_values(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        let sql = r#"CREATE TYPE "invalid" as ENUM ('@', '-')"#;
        api.database().execute_raw(sql, &[]).await?;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());

                let typ = if sql_family.is_postgres() {
                    "invalid"
                } else {
                    "ENUM ('@', '-')"
                };

                t.add_column("sign", types::custom(typ).nullable(false));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let enum_name = if sql_family.is_postgres() {
        "invalid"
    } else {
        "User_sign"
    };

    let input_dm = format!(
        r#"
        model User {{
            id               Int @id @default(autoincrement())
            sign             {0}
        }}

        enum {0} {{
            at      @map("@")
            dash    @map("-")
        }}
    "#,
        enum_name
    );

    let final_dm = format!(
        r#"
        model User {{
            id               Int @id @default(autoincrement())
            sign             {0}
        }}

        model Unrelated {{
            id               Int @id @default(autoincrement())
        }}

        enum {0} {{
            at      @map("@")
            dash    @map("-")
        }}
    "#,
        enum_name
    );

    assert_eq_datamodels!(&final_dm, &api.re_introspect(&input_dm).await?);

    let expected = json!([{
        "code": 10,
        "message": "These enum values were enriched with `@map` information taken from the previous Prisma schema.",
        "affected" :[
            {"enm": "invalid", "value": "at"},
            {"enm": "invalid", "value": "dash"}
        ]
    }]);

    assert_eq_json!(expected, api.re_introspect_warnings(&input_dm).await?);

    Ok(())
}

#[test_each_connector]
async fn multiple_changed_relation_names(api: &TestApi) -> crate::TestResult {
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

    let (idx1, idx2) = if api.sql_family().is_mysql() {
        (
            r#"@@index([eveningEmployeeId], name: "eveningEmployeeId")"#,
            r#"@@index([morningEmployeeId], name: "morningEmployeeId")"#,
        )
    } else {
        ("", "")
    };

    let input_dm = format!(
        r#"
        model Employee {{
            id                                            Int         @id @default(autoincrement())
            A                                             Schedule[]  @relation("EmployeeToSchedule_eveningEmployeeId")
            Schedule_EmployeeToSchedule_morningEmployeeId Schedule[]  @relation("EmployeeToSchedule_morningEmployeeId")
        }}

        model Schedule {{
            id                                            Int         @id @default(autoincrement())
            morningEmployeeId                             Int
            eveningEmployeeId                             Int
            Employee_EmployeeToSchedule_eveningEmployeeId Employee    @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id])
            Employee_EmployeeToSchedule_morningEmployeeId Employee    @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id])
            {}
            {}
        }}
    "#,
        idx1, idx2
    );

    let final_dm = format!(
        r#"
        model Employee {{
            id                                            Int         @id @default(autoincrement())
            A                                             Schedule[]  @relation("EmployeeToSchedule_eveningEmployeeId")
            Schedule_EmployeeToSchedule_morningEmployeeId Schedule[]  @relation("EmployeeToSchedule_morningEmployeeId")
        }}

        model Schedule {{
            id                                            Int         @id @default(autoincrement())
            morningEmployeeId                             Int
            eveningEmployeeId                             Int
            Employee_EmployeeToSchedule_eveningEmployeeId Employee    @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id])
            Employee_EmployeeToSchedule_morningEmployeeId Employee    @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id])
            {}
            {}
        }}

        model Unrelated {{
            id               Int @id @default(autoincrement())
        }}
    "#,
        idx1, idx2
    );

    assert_eq_datamodels!(&final_dm, &api.re_introspect(&input_dm).await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn custom_virtual_relation_field_names(api: &TestApi) -> crate::TestResult {
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

    let final_dm = indoc! {r#"
        model Post {
            id               Int @id @default(autoincrement())
            user_id          Int  @unique
            custom_User      User @relation(fields: [user_id], references: [id])
        }

        model User {
            id               Int @id @default(autoincrement())
            custom_Post      Post?
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }

    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    Ok(())
}

#[test_each_connector]
async fn custom_model_order(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("A", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("B", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("J", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("F", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Z", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("M", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("L", |t| {
                t.add_column("id", types::primary());
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

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn custom_enum_order(api: &TestApi) -> crate::TestResult {
    let sql = "CREATE Type a as ENUM ( \'id\')".to_string();
    api.database().execute_raw(&sql, &[]).await?;

    let sql = "CREATE Type b as ENUM ( \'id\')".to_string();
    api.database().execute_raw(&sql, &[]).await?;

    let sql = "CREATE Type j as ENUM ( \'id\')".to_string();
    api.database().execute_raw(&sql, &[]).await?;

    let sql = "CREATE Type f as ENUM ( \'id\')".to_string();
    api.database().execute_raw(&sql, &[]).await?;

    let sql = "CREATE Type z as ENUM ( \'id\')".to_string();
    api.database().execute_raw(&sql, &[]).await?;

    let sql = "CREATE Type m as ENUM ( \'id\')".to_string();
    api.database().execute_raw(&sql, &[]).await?;

    let sql = "CREATE Type l as ENUM ( \'id\')".to_string();
    api.database().execute_raw(&sql, &[]).await?;

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

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    Ok(())
}

#[test_each_connector]
async fn multiple_changed_relation_names_due_to_mapped_models(api: &TestApi) -> crate::TestResult {
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
            custom_User      Custom_User @relation("CustomRelationName", fields: [user_id], references: [id])
            custom_User2     Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id])
        }

        model Custom_User {
            id               Int @id @default(autoincrement())
            custom_Post      Post? @relation("CustomRelationName")
            custom_Post2     Post? @relation("AnotherCustomRelationName")

            @@map("User")
        }
    "#};

    let final_dm = indoc! {r#"
        model Post {
            id               Int @id @default(autoincrement())
            user_id          Int  @unique
            user_id2         Int  @unique
            custom_User      Custom_User @relation("CustomRelationName", fields: [user_id], references: [id])
            custom_User2     Custom_User @relation("AnotherCustomRelationName", fields: [user_id2], references: [id])
        }

        model Custom_User {
            id               Int @id @default(autoincrement())
            custom_Post      Post? @relation("CustomRelationName")
            custom_Post2     Post? @relation("AnotherCustomRelationName")

            @@map("User")
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }
    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(&input_dm).await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn virtual_cuid_default(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::varchar(30).primary(true));
                t.add_column("non_id", types::varchar(30));
            });

            migration.create_table("User2", |t| {
                t.add_column("id", types::varchar(30).primary(true));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model User {
            id        String    @id @default(cuid())
            non_id    String    @default(cuid())
        }

        model User2 {
            id        String    @id @default(uuid())
        }
    "#};

    let final_dm = indoc! {r#"
        model User {
            id        String    @id @default(cuid())
            non_id    String    @default(cuid())
        }

        model User2 {
            id        String    @id @default(uuid())
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }
    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn comments(api: &TestApi) -> crate::TestResult {
    let sql = "CREATE Type a as ENUM (\'A\')".to_string();
    api.database().execute_raw(&sql, &[]).await?;

    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::varchar(30).primary(true));
            });

            migration.create_table("User2", |t| {
                t.add_column("id", types::varchar(30).primary(true));
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
            id        String    @id @default(cuid())
        }

        model User2 {
            id        String    @id @default(uuid())
        }

        /// A really helpful comment about the enum
        enum a {
            A // A really helpful comment about enum variant
        }

        /// just floating around here
    "#};

    let final_dm = indoc! {r#"
        /// A really helpful comment about the model
        model User {
            /// A really helpful comment about the field
            id        String    @id @default(cuid())
        }

        model User2 {
            id        String    @id @default(uuid())
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }

        /// A really helpful comment about the enum
        enum a {
            A // A really helpful comment about enum variant
        }

        /// just floating around here
    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    Ok(())
}

#[test_each_connector]
async fn updated_at(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::varchar(30).primary(true));
                t.add_column("lastupdated", types::datetime().nullable(true));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let input_dm = indoc! {r#"
        model User {
            id           String    @id
            lastupdated  DateTime? @updatedAt
        }
    "#};

    let final_dm = indoc! {r#"
        model User {
            id           String    @id
            lastupdated  DateTime? @updatedAt
        }

        model Unrelated {
            id               Int @id @default(autoincrement())
        }
    "#};

    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);

    Ok(())
}

#[test_each_connector]
async fn multiple_many_to_many_on_same_model(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("A", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("B", |t| {
                t.add_column("id", types::primary());
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
                t.add_column("id", types::primary());
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

    let final_dm = indoc! {r#"
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

        model Unrelated {
            id Int @id @default(autoincrement())
        }
    "#};

    assert_eq_datamodels!(&final_dm, &api.re_introspect(&input_dm).await?);

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn re_introspecting_mysql_enum_names(api: &TestApi) -> crate::TestResult {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  ENUM('black', 'white') Not Null");
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

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

    let final_dm = r#"
             model User {
               id               Int @id @default(autoincrement())
               color            BlackNWhite
            }

            model Unrelated {
               id               Int @id @default(autoincrement())
            }

            enum BlackNWhite{
                black
                white
            }
        "#;
    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);
    assert_eq_json!(
        serde_json::Value::Array(vec![]),
        &api.re_introspect_warnings(input_dm).await?
    );

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn re_introspecting_mysql_enum_names_if_enum_is_reused(api: &TestApi) -> crate::TestResult {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  ENUM('black', 'white') Not Null");
                t.inject_custom("color2  ENUM('black', 'white') Not Null");
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

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

    let final_dm = r#"
             model User {
               id               Int @id @default(autoincrement())
               color            BlackNWhite
               color2           User_color2
            }

            model Unrelated {
               id               Int @id @default(autoincrement())
            }

            enum BlackNWhite{
                black
                white
            }
            
            enum User_color2{
                black
                white
            }
        "#;
    assert_eq_datamodels!(final_dm, &api.re_introspect(input_dm).await?);
    assert_eq_json!(
        serde_json::Value::Array(vec![]),
        &api.re_introspect_warnings(input_dm).await?
    );

    Ok(())
}
