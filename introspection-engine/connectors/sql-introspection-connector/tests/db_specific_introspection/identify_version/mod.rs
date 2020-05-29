use crate::*;
use barrel::types;
use introspection_connector::Version;
use test_harness::*;

//todo adjust tests for added types
//todo adjust tests for new singular id rule for p1

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
async fn introspect_sqlite_non_prisma_due_to_types(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("_Migration", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("point geometric");
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
                t.inject_custom("location   point");
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::NonPrisma);
}

#[test_each_connector(tags("postgres"))]
async fn introspect_postgres_prisma_1(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("createdAt timestamp(3)");
                t.inject_custom("updatedAt timestamp(3)");
                t.inject_custom("string text");
                t.inject_custom("int Integer");
                t.inject_custom("float Decimal(65,30)");
                t.inject_custom("boolean boolean");
            });
            migration.create_table("_RelayId", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("stableModelIdentifier   Integer");
            });

            migration.create_table("Book_tags", |t| {
                t.add_column("nodeid", types::primary());
                t.add_column("position", types::integer());
                t.add_column("value", types::integer());
                t.inject_custom("FOREIGN KEY (\"nodeid\") REFERENCES \"Book\"(\"id\")");
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::Prisma1);
}

#[test_each_connector(tags("postgres"))]
async fn introspect_postgres_prisma_1_1(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("date timestamp(3)");
                t.inject_custom("string text");
                t.inject_custom("int Integer");
                t.inject_custom("float Decimal(65,30)");
                t.inject_custom("boolean boolean");
            });

            migration.create_table("Page", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("string text");
                t.add_column("bookid", types::integer());
                t.inject_custom("FOREIGN KEY (\"bookid\") REFERENCES \"Book\"(\"id\")");
            });

            migration.create_table("_RelayId", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("stableModelIdentifier   Integer");
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::Prisma11);
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

//Mysql

#[test_each_connector(tags("mysql"))]
async fn introspect_mysql_non_prisma(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("location   point");
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::NonPrisma);
}

#[test_each_connector(tags("mysql"))]
async fn introspect_mysql_prisma_1(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("createdAt datetime(3)");
                t.inject_custom("updatedAt datetime(3)");
                t.inject_custom("string_column text");
                t.inject_custom("integer_column int");
                t.inject_custom("float_column Decimal(65,30)");
                t.inject_custom("boolean_column boolean");
            });
            migration.create_table("_RelayId", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("stableModelIdentifier   int");
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::Prisma1);
}

#[test_each_connector(tags("mysql"))]
async fn introspect_mysql_prisma_1_1(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("datetime_column datetime(3)");
                t.inject_custom("string_column text");
                t.inject_custom("integer_column int");
                t.inject_custom("float_column Decimal(65,30)");
                t.inject_custom("boolean_column boolean");
            });
            migration.create_table("_RelayId", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("stableModelIdentifier   int");
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::Prisma11);
}

#[test_each_connector(tags("mysql"))]
async fn introspect_mysql_prisma2(api: &TestApi) {
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
