use barrel::types;
use indoc::indoc;
use introspection_engine_tests::{assert_eq_datamodels, test_api::*};
use quaint::prelude::{Queryable, SqlFamily};
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector(capabilities("enums"))]
async fn a_table_with_enums(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.database()
            .raw_cmd(r#"CREATE TYPE "color" AS ENUM ('black', 'white')"#)
            .await?;

        api.database()
            .raw_cmd(r#"CREATE TYPE "color2" AS ENUM ('black2', 'white2')"#)
            .await?;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let (typ1, typ2) = if sql_family.is_mysql() {
                    ("ENUM ('black', 'white')", "ENUM ('black2', 'white2')")
                } else {
                    ("color", "color2")
                };

                t.add_column("color", types::custom(typ1).nullable(false));
                t.add_column("color2", types::custom(typ2).nullable(false));
            });
        })
        .await?;

    let color = if sql_family.is_mysql() { "Book_color" } else { "color" };

    let color2 = if sql_family.is_mysql() { "Book_color2" } else { "color2" };

    let dm = format!(
        r#"
        model Book {{
            id      Int     @id @default(autoincrement())
            color   {0}
            color2  {1}
        }}

        enum {0} {{
            black
            white
        }}

        enum {1} {{
            black2
            white2
        }}
    "#,
        color, color2
    );

    for _ in 0..4 {
        assert_eq_datamodels!(&dm, &api.introspect().await?);
    }

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn a_table_with_an_enum_default_value_that_is_an_empty_string(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.database()
            .raw_cmd(r#"CREATE TYPE "color" AS ENUM ('black', '')"#)
            .await?;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let typ = if sql_family.is_mysql() {
                    "ENUM ('black', '')"
                } else {
                    "color"
                };

                t.add_column("color", types::custom(typ).default("").nullable(false));
            });
        })
        .await?;

    let color = if sql_family.is_mysql() { "Book_color" } else { "color" };

    let dm = format!(
        r#"
        model Book {{
            id      Int @id @default(autoincrement())
            color   {0}     @default(EMPTY_ENUM_VALUE)
        }}

        enum {0} {{
            black
            EMPTY_ENUM_VALUE @map("")
        }}
    "#,
        color
    );

    assert_eq_datamodels!(&dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn a_table_enums_should_return_alphabetically_even_when_in_different_order(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.database()
            .raw_cmd(r#"CREATE TYPE "color2" AS ENUM ('black2', 'white2')"#)
            .await?;

        api.database()
            .raw_cmd(r#"CREATE TYPE "color" AS ENUM ('black', 'white')"#)
            .await?;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let (typ1, typ2) = if sql_family.is_mysql() {
                    ("ENUM ('black', 'white')", "ENUM ('black2', 'white2')")
                } else {
                    ("color", "color2")
                };

                t.add_column("color", types::custom(typ1).nullable(false));
                t.add_column("color2", types::custom(typ2).nullable(false));
            });
        })
        .await?;

    let color = if sql_family.is_mysql() { "Book_color" } else { "color" };
    let color2 = if sql_family.is_mysql() { "Book_color2" } else { "color2" };

    let dm = format!(
        r#"
        model Book {{
            id      Int     @id @default(autoincrement())
            color   {1}
            color2  {0}
        }}

        enum {1} {{
            black
            white
        }}

        enum {0} {{
            black2
            white2
        }}
    "#,
        color2, color
    );

    for _ in 0..4 {
        assert_eq_datamodels!(&dm, &api.introspect().await?);
    }

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn a_table_with_enum_default_values(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.database()
            .raw_cmd(r#"CREATE TYPE "color" AS ENUM ('black', 'white')"#)
            .await?;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let typ = if sql_family.is_mysql() {
                    "ENUM ('black', 'white')"
                } else {
                    "color"
                };

                t.add_column("color", types::custom(typ).nullable(false).default("black"));
            });
        })
        .await?;

    let enum_name = if sql_family.is_mysql() { "Book_color" } else { "color" };

    let dm = format!(
        r#"
        model Book {{
            id      Int @id @default(autoincrement())
            color   {0} @default(black)
        }}

        enum {0} {{
            black
            white
        }}
    "#,
        enum_name
    );

    assert_eq_datamodels!(&dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(capabilities("enums", "scalar_lists"))]
async fn a_table_enums_array(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    match sql_family {
        SqlFamily::Postgres => {
            api.database()
                .raw_cmd(r#"CREATE Type "color" as ENUM ('black','white')"#)
                .await?;
        }
        _ => todo!("{}", sql_family),
    }

    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.add_column("color", types::custom("color[]"));
            });
        })
        .await?;

    let dm = indoc! {r#"
        datasource pg {
            provider = "postgres"
            url = "postgresql://localhost:5432"
        }

        model Book {
            id      Int     @id @default(autoincrement())
            color   color[]
        }

        enum color {
            black
            white
        }
    "#};

    let result = format!(
        r#"
        datasource pg {{
            provider = "postgres"
            url = "postgresql://localhost:5432"
        }}

        {}
    "#,
        api.introspect().await?
    );

    assert_eq_datamodels!(&dm, &result);

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn a_table_with_enum_default_values_that_look_like_booleans(api: &TestApi) -> crate::TestResult {
    let sql_family = api.sql_family();

    if sql_family.is_postgres() {
        api.database()
            .raw_cmd("CREATE Type truth as ENUM ('true', 'false', 'rumor')")
            .await?;
    }

    api.barrel()
        .execute(move |migration| {
            migration.create_table("News", move |t| {
                t.add_column("id", types::primary());

                let typ = if sql_family.is_postgres() {
                    "truth"
                } else {
                    "ENUM ('true', 'false', 'rumor')"
                };

                t.add_column("confirmed", types::custom(typ).nullable(false).default("true"));
            });
        })
        .await?;

    let enum_name = if sql_family.is_mysql() {
        "News_confirmed"
    } else {
        "truth"
    };

    let dm = format!(
        r#"
        model News {{
            id          Int @id @default(autoincrement())
            confirmed   {0} @default(true)
        }}

        enum {0} {{
            true
            false
            rumor
        }}
    "#,
        enum_name
    );

    assert_eq_datamodels!(&dm, &api.introspect().await?);

    Ok(())
}
