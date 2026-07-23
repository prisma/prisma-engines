use barrel::types;
use indoc::indoc;
use quaint::prelude::Queryable;
use sql_introspection_tests::{TestResult, test_api::*};
use test_macros::test_connector;

#[test_connector(tags(CockroachDb), capabilities(Enums))]
async fn a_table_with_enums(api: &mut TestApi) -> TestResult {
    api.database()
        .raw_cmd(r#"CREATE TYPE "color" AS ENUM ('black', 'white')"#)
        .await?;

    api.database()
        .raw_cmd(r#"CREATE TYPE "color2" AS ENUM ('black2', 'white2')"#)
        .await?;

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let (typ1, typ2) = ("color", "color2");

                t.add_column("color", types::custom(typ1).nullable(false));
                t.add_column("color2", types::custom(typ2).nullable(false));
            });
        })
        .await?;

    let dm = r#"
        model Book {
            id      BigInt     @id @default(autoincrement())
            color   color
            color2  color2
        }

        enum color {
            black
            white
        }

        enum color2 {
            black2
            white2
        }
    "#;

    for _ in 0..4 {
        let result = api.introspect().await?;
        api.assert_eq_datamodels(dm, &result);
    }

    Ok(())
}

#[test_connector(tags(CockroachDb), capabilities(Enums))]
async fn a_table_with_an_enum_default_value_that_is_an_empty_string(api: &mut TestApi) -> TestResult {
    api.database()
        .raw_cmd(r#"CREATE TYPE "color" AS ENUM ('black', '')"#)
        .await?;

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let typ = "color";

                t.add_column("color", types::custom(typ).default("").nullable(false));
            });
        })
        .await?;

    let dm = format!(
        r#"
        model Book {{
            id      BigInt @id @default(autoincrement())
            color   {0}     @default(EMPTY_ENUM_VALUE)
        }}

        enum {0} {{
            black
            EMPTY_ENUM_VALUE @map("")
        }}
    "#,
        "color",
    );

    let result = api.introspect().await?;
    api.assert_eq_datamodels(&dm, &result);

    Ok(())
}

#[test_connector(tags(CockroachDb), capabilities(Enums))]
async fn a_table_enums_should_return_alphabetically_even_when_in_different_order(api: &mut TestApi) -> TestResult {
    api.database()
        .raw_cmd(r#"CREATE TYPE "color2" AS ENUM ('black2', 'white2')"#)
        .await?;

    api.database()
        .raw_cmd(r#"CREATE TYPE "color" AS ENUM ('black', 'white')"#)
        .await?;

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let (typ1, typ2) = ("color", "color2");

                t.add_column("color", types::custom(typ1).nullable(false));
                t.add_column("color2", types::custom(typ2).nullable(false));
            });
        })
        .await?;

    let dm = format!(
        r#"
        model Book {{
            id      BigInt     @id @default(autoincrement())
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
        "color2", "color"
    );

    for _ in 0..4 {
        let result = api.introspect().await?;
        api.assert_eq_datamodels(&dm, &result);
    }

    Ok(())
}

#[test_connector(tags(CockroachDb), capabilities(Enums))]
async fn a_table_with_enum_default_values(api: &mut TestApi) -> TestResult {
    api.database()
        .raw_cmd(r#"CREATE TYPE "color" AS ENUM ('black', 'white')"#)
        .await?;

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::primary());

                let typ = "color";

                t.add_column("color", types::custom(typ).nullable(false).default("black"));
            });
        })
        .await?;

    let dm = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        model Book {
          id    BigInt @id @default(autoincrement())
          color color  @default(black)
        }

        enum color {
          black
          white
        }
    "#]];

    api.expect_datamodel(&dm).await;

    Ok(())
}

#[test_connector(tags(CockroachDb), capabilities(Enums, ScalarLists))]
async fn a_table_enums_array(api: &mut TestApi) -> TestResult {
    api.database()
        .raw_cmd(r#"CREATE Type "color" as ENUM ('black','white')"#)
        .await?;

    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.add_column("color", types::custom("color[]"));
            });
        })
        .await?;

    let dm = indoc! {
        r#"
        model Book {
            id      BigInt     @id @default(autoincrement())
            color   color[]
        }

        enum color {
            black
            white
        }
        "#,
    };

    let result = api.introspect().await?;

    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(CockroachDb), capabilities(Enums))]
async fn a_table_with_enum_default_values_that_look_like_booleans(api: &mut TestApi) -> TestResult {
    api.database()
        .raw_cmd("CREATE Type truth as ENUM ('true', 'false', 'rumor')")
        .await?;

    api.barrel()
        .execute(move |migration| {
            migration.create_table("News", move |t| {
                t.add_column("id", types::primary());

                let typ = "truth";
                t.add_column("confirmed", types::custom(typ).nullable(false).default("true"));
            });
        })
        .await?;

    let enum_name = "truth";

    let dm = format!(
        r#"
        model News {{
            id          BigInt @id @default(autoincrement())
            confirmed   {enum_name} @default(true)
        }}

        enum {enum_name} {{
            true
            false
            rumor
        }}
    "#,
    );

    let result = api.introspect().await?;
    api.assert_eq_datamodels(&dm, &result);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn an_enum_with_invalid_value_names_should_have_them_commented_out(api: &mut TestApi) -> TestResult {
    let sql = r#"CREATE TYPE "threechars" AS ENUM ('123', 'wow','$§!');"#;
    api.raw_cmd(sql).await;
    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        enum threechars {
          // 123 @map("123")
          wow
          // $§! @map("$§!")
        }
    "#]];
    api.expect_datamodel(&expected).await;
    Ok(())
}

// Regression: https://github.com/prisma/prisma/issues/22456
#[test_connector(tags(CockroachDb))]
async fn enum_array_type(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TYPE "_foo" AS ENUM ('FIRST', 'SECOND');

        CREATE TABLE "Post" (
            "id" TEXT NOT NULL,
            "contentFilters" "_foo"[],
            CONSTRAINT "Post_pkey" PRIMARY KEY ("id")
        );
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "cockroachdb"
        }

        model Post {
          id             String @id
          contentFilters foo[]
        }

        enum foo {
          FIRST
          SECOND

          @@map("_foo")
        }
    "#]];

    api.expect_datamodel(&expectation).await;
    api.expect_no_warnings().await;

    Ok(())
}
