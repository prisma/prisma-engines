use barrel::types;
use expect_test::expect;
use introspection_engine_tests::{test_api::*, TestResult};
use test_macros::test_connector;

#[test_connector(tags(Postgres))]
async fn add_cuid_default_postgres(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::varchar(25).nullable(false).primary(true));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Book {
          id String @id @default(cuid()) @db.VarChar(25)
        }
    "#]];
    expected.assert_eq(&api.introspect_dml().await?);

    let expected = expect![[
        r#"[{"code":5,"message":"These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1.","affected":[{"model":"Book","field":"id"}]}]"#
    ]];
    expected.assert_eq(&api.introspection_warnings().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn add_cuid_default_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::r#char(25).nullable(false).primary(true));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Book {
          id String @id @default(cuid()) @db.Char(25)
        }
    "#]];
    expected.assert_eq(&api.introspect_dml().await?);

    let expected = expect![[
        r#"[{"code":5,"message":"These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1.","affected":[{"model":"Book","field":"id"}]}]"#
    ]];
    expected.assert_eq(&api.introspection_warnings().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn add_uuid_default_postgres(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::varchar(36).nullable(false).primary(true));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Book {
          id String @id @default(uuid()) @db.VarChar(36)
        }
    "#]];
    expected.assert_eq(&api.introspect_dml().await?);

    let expected = expect![[
        r#"[{"code":6,"message":"These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1.","affected":[{"model":"Book","field":"id"}]}]"#
    ]];
    expected.assert_eq(&api.introspection_warnings().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn add_uuid_default_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Book", move |t| {
                t.add_column("id", types::r#char(36).nullable(false).primary(true));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Book {
          id String @id @default(uuid()) @db.Char(36)
        }
    "#]];
    expected.assert_eq(&api.introspect_dml().await?);

    let expected = expect![[
        r#"[{"code":6,"message":"These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1.","affected":[{"model":"Book","field":"id"}]}]"#
    ]];
    expected.assert_eq(&api.introspection_warnings().await?);

    Ok(())
}
