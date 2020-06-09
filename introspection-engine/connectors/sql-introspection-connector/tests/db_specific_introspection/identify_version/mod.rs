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
async fn introspect_sqlite_non_prisma_due_to_types(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("_Migration", |t| {
                t.add_column("name", types::primary());
                t.add_column("revision", types::text());
                t.add_column("datamodel", types::text());
                t.add_column("status", types::text());
                t.add_column("applied", types::text());
                t.add_column("rolled_back", types::text());
                t.add_column("datamodel_steps", types::text());
                t.add_column("database_migration", types::text());
                t.add_column("errors", types::text());
                t.add_column("started_at", types::text());
                t.add_column("finished_at", types::text());
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
                t.add_column("name", types::primary());
                t.add_column("revision", types::text());
                t.add_column("datamodel", types::text());
                t.add_column("status", types::text());
                t.add_column("applied", types::text());
                t.add_column("rolled_back", types::text());
                t.add_column("datamodel_steps", types::text());
                t.add_column("database_migration", types::text());
                t.add_column("errors", types::text());
                t.add_column("started_at", types::text());
                t.add_column("finished_at", types::text());
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
                t.inject_custom("id character varying(25) Not Null Primary Key");
                t.inject_custom("createdAt timestamp(3)");
                t.inject_custom("updatedAt timestamp(3)");
                t.inject_custom("string text");
                t.inject_custom("int Integer");
                t.inject_custom("float Decimal(65,30)");
                t.inject_custom("boolean boolean");
            });
            migration.create_table("_RelayId", |t| {
                t.inject_custom("id character varying(25) Primary Key ");
                t.inject_custom("stableModelIdentifier character varying(25) Not Null");
            });

            migration.create_table("Book_tags", |t| {
                t.inject_custom("nodeid character varying(25) references \"Book\"(\"id\")");
                t.inject_custom("position integer");
                t.inject_custom("value integer NOT NULL");
                t.inject_custom("CONSTRAINT \"BookTags_list_pkey\" PRIMARY KEY (\"nodeid\", \"position\")");
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
                t.inject_custom("id character varying(36) Not Null Primary Key");
                t.inject_custom("date timestamp(3)");
                t.inject_custom("string text");
                t.inject_custom("int Integer");
                t.inject_custom("float Decimal(65,30)");
                t.inject_custom("boolean boolean");
            });

            migration.create_table("Page", |t| {
                t.inject_custom("id character varying(36) Not Null Primary Key");
                t.inject_custom("string text");
                t.inject_custom("bookid character varying(36) REFERENCES \"Book\"(\"id\")");
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
                t.add_column("name", types::primary());
                t.add_column("revision", types::text());
                t.add_column("datamodel", types::text());
                t.add_column("status", types::text());
                t.add_column("applied", types::text());
                t.add_column("rolled_back", types::text());
                t.add_column("datamodel_steps", types::text());
                t.add_column("database_migration", types::text());
                t.add_column("errors", types::text());
                t.add_column("started_at", types::text());
                t.add_column("finished_at", types::text());
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
                t.inject_custom("id char(25) Not Null Primary Key");
                t.inject_custom("createdAt datetime(3)");
                t.inject_custom("updatedAt datetime(3)");
                t.inject_custom("string_column text");
                t.inject_custom("integer_column int");
                t.inject_custom("float_column Decimal(65,30)");
                t.inject_custom("boolean_column boolean");
            });
            migration.create_table("_RelayId", |t| {
                t.inject_custom("id char(25) Not Null Primary Key");
                t.inject_custom("stableModelIdentifier   char(25)");
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
                t.inject_custom("id char(36) Not Null Primary Key");
                t.inject_custom("datetime_column datetime(3)");
                t.inject_custom("string_column text");
                t.inject_custom("integer_column int");
                t.inject_custom("float_column Decimal(65,30)");
                t.inject_custom("boolean_column boolean");
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
                t.add_column("name", types::primary());
                t.add_column("revision", types::text());
                t.add_column("datamodel", types::text());
                t.add_column("status", types::text());
                t.add_column("applied", types::text());
                t.add_column("rolled_back", types::text());
                t.add_column("datamodel_steps", types::text());
                t.add_column("database_migration", types::text());
                t.add_column("errors", types::text());
                t.add_column("started_at", types::text());
                t.add_column("finished_at", types::text());
            });
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let result = dbg!(api.introspect_version().await);
    assert_eq!(result, Version::Prisma2);
}
