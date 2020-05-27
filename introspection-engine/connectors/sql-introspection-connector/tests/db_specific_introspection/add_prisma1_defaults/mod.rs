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
                id  String @default(cuid()) @id
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
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
                id  String @default(cuid()) @id
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn add_cuid_default_for_mysql(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("id varchar(25) Not Null Primary Key");
            });
        })
        .await;

    let dm = r#"
            model Book {
                id  String @default(cuid()) @id
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn add_uuid_default_for_mysql(api: &TestApi) {
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
}
