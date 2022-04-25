use barrel::types;
use introspection_connector::Version;
use introspection_engine_tests::{test_api::*, TestResult};
use pretty_assertions::assert_eq;
use test_macros::test_connector;

//Sqlite
#[test_connector(tags(Sqlite))]
async fn introspect_sqlite_non_prisma(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    assert_eq!(Version::NonPrisma, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn sqlite_non_prisma_due_to_types(api: &TestApi) -> TestResult {
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
        .await?;

    assert_eq!(Version::NonPrisma, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn introspect_sqlite_prisma2(api: &TestApi) -> TestResult {
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
        .await?;

    assert_eq!(Version::Prisma2, api.introspect_version().await?);

    Ok(())
}

//Postgres

#[test_connector(tags(Postgres))]
async fn introspect_postgres_non_prisma(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("location   INT2");
            });
        })
        .await?;

    assert_eq!(Version::NonPrisma, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn introspect_postgres_prisma_1(api: &TestApi) -> TestResult {
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
        .await?;

    assert_eq!(Version::Prisma1, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn introspect_cockroach_prisma_1(api: &TestApi) -> TestResult {
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
        .await?;

    assert_eq!(Version::NonPrisma, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn introspect_postgres_prisma_1_1(api: &TestApi) -> TestResult {
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
        .await?;

    assert_eq!(Version::Prisma11, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn introspect_postgres_prisma2(api: &TestApi) -> TestResult {
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
        .await?;

    assert_eq!(Version::Prisma2, api.introspect_version().await?);

    Ok(())
}

//Mysql

#[test_connector(tags(Mysql))]
async fn introspect_mysql_non_prisma(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("location   point");
            });
        })
        .await?;

    assert_eq!(Version::NonPrisma, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn introspect_mysql_prisma_1(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("id char(25) Not Null Primary Key");
                t.inject_custom("createdAt datetime(3)");
                t.inject_custom("updatedAt datetime(3)");
                t.inject_custom("string_column mediumtext");
                t.inject_custom("integer_column int(11)");
                t.inject_custom("float_column Decimal(65,30)");
                t.inject_custom("boolean_column boolean");
            });
            migration.create_table("_RelayId", |t| {
                t.inject_custom("id char(25) Not Null Primary Key");
                t.inject_custom("stableModelIdentifier   char(25)");
            });
        })
        .await?;

    assert_eq!(Version::Prisma1, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn introspect_mysql_prisma_1_1_if_not_for_default_value(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("id char(25) Not Null Primary Key");
                t.inject_custom("string_column mediumtext");
                t.inject_custom("integer_column int(11) DEFAULT 5");
                t.inject_custom("float_column Decimal(65,30)");
                t.inject_custom("boolean_column boolean");
            });
        })
        .await?;

    assert_eq!(Version::NonPrisma, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn introspect_mysql_prisma_1_1(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("id char(36) Not Null Primary Key");
                t.inject_custom("datetime_column datetime(3)");
                t.inject_custom("string_column mediumtext");
                t.inject_custom("integer_column int(11)");
                t.inject_custom("float_column Decimal(65,30)");
                t.inject_custom("boolean_column boolean");
            });
        })
        .await?;

    assert_eq!(Version::Prisma11, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn introspect_mysql_prisma2(api: &TestApi) -> TestResult {
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
                t.inject_custom("id char(36) Not Null Primary Key");
            });
        })
        .await?;

    assert_eq!(Version::Prisma2, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn introspect_mysql_non_prisma_empty(api: &TestApi) -> TestResult {
    api.barrel().execute(|_migration| {}).await?;

    assert_eq!(Version::NonPrisma, api.introspect_version().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn introspect_mssql_non_prisma_empty(api: &TestApi) -> TestResult {
    api.barrel().execute(|_migration| {}).await?;

    assert_eq!(Version::NonPrisma, api.introspect_version().await?);

    Ok(())
}
