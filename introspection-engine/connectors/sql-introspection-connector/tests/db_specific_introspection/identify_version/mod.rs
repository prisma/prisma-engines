use crate::*;
use barrel::types;
use introspection_connector::Version;
use test_harness::*;

//Sqlite
#[test_each_connector(tags("sqlite"))]
async fn introspect_sqlite_non_prisma(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::NonPrisma);
}

#[test_each_connector(tags("sqlite"))]
async fn introspect_sqlite_prisma2(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("_Migration", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::Prisma2);
}

//Postgres

#[test_each_connector(tags("postgres"))]
async fn introspect_postgres_non_prisma(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::NonPrisma);
}

#[test_each_connector(tags("postgres"))]
async fn introspect_postgres_prisma2(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("_Migration", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::Prisma2);
}

//Prisma1
//Prisma11
//Prisma2
