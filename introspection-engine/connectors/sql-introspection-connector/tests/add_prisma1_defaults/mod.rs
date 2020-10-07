use crate::*;
use test_harness::*;

#[test_each_connector(tags("postgres"))]
async fn add_cuid_default_for_postgres(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("id varchar(25) Not Null Primary Key");
            });
        })
        .await;

    let dm = r#"
            model Book {
                id  String @id @default(cuid())
            }
        "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(&warnings, "[{\"code\":5,\"message\":\"These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1.\",\"affected\":[{\"model\":\"Book\",\"field\":\"id\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn add_uuid_default_for_postgres(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("id varchar(36) Not Null Primary Key");
            });
        })
        .await;

    let dm = r#"
            model Book {
                id  String @default(uuid()) @id
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(&warnings, "[{\"code\":6,\"message\":\"These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1.\",\"affected\":[{\"model\":\"Book\",\"field\":\"id\"}]}]");
}

#[test_each_connector(tags("mysql"))]
async fn add_cuid_default_for_mysql(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("id char(25) Not Null Primary Key");
            });
        })
        .await;

    let dm = r#"
            model Book {
                id  String @id @default(cuid())
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(&warnings, "[{\"code\":5,\"message\":\"These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1.\",\"affected\":[{\"model\":\"Book\",\"field\":\"id\"}]}]");
}

#[test_each_connector(tags("mysql"))]
async fn add_uuid_default_for_mysql(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("id char(36) Not Null Primary Key");
            });
        })
        .await;

    let dm = r#"
            model Book {
                id  String @default(uuid()) @id
            }

        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(&warnings, "[{\"code\":6,\"message\":\"These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1.\",\"affected\":[{\"model\":\"Book\",\"field\":\"id\"}]}]");
}
