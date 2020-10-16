use barrel::{functions, types};
use indoc::indoc;
use introspection_engine_tests::{assert_eq_datamodels, test_api::*};
use quaint::prelude::Queryable;
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector]
async fn a_simple_table_with_gql_types(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("bool", types::boolean());
                    t.add_column("float", types::float());
                    t.add_column("date", types::datetime());
                    t.add_column("id", types::primary());
                    t.add_column("int", types::integer());
                    t.add_column("string", types::text());
                });

                migration.create_table("_RelayId", |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom("stableModelIdentifier   int");
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Blog {
            bool    Boolean
            float   Float
            date    DateTime
            id      Int @id @default(autoincrement())
            int     Int
            string  String
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_table_with_compound_primary_keys(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::integer());
                    t.add_column("authorId", types::varchar(10));
                    t.set_primary_key(&["id", "authorId"]);
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Blog {
            id Int
            authorId String
            @@id([id, authorId])
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_table_with_unique_index(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("authorId", types::r#char(10));
                    t.add_index("test", types::index(vec!["authorId"]).unique(true));
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Blog {
            id      Int @id @default(autoincrement())
            authorId String @unique
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_table_with_multi_column_unique_index(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("firstname", types::varchar(10));
                    t.add_column("lastname", types::varchar(10));
                    t.add_index("test", types::index(vec!["firstname", "lastname"]).unique(true));
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model User {
            id      Int @id @default(autoincrement())
            firstname String
            lastname String
            @@unique([firstname, lastname], name: "test")
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_table_with_required_and_optional_columns(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("requiredname", types::varchar(255).nullable(false));
                    t.add_column("optionalname", types::varchar(255).nullable(true));
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model User {
            id      Int @id @default(autoincrement())
            requiredname String
            optionalname String?
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_table_with_default_values(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("a", types::text());
                    t.add_column("id", types::primary());
                    t.add_column("bool", types::boolean().default(false).nullable(false));
                    t.add_column("bool2", types::boolean().default(true).nullable(false));
                    t.add_column("float", types::float().default(5.3).nullable(false));
                    t.add_column("int", types::integer().default(5).nullable(false));
                    t.add_column("string", types::varchar(4).default("Test").nullable(false));
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model User {
            a String
            id     Int     @id @default(autoincrement())
            bool   Boolean @default(false)
            bool2  Boolean @default(true)
            float  Float   @default(5.3)
            int    Int     @default(5)
            string String  @default("Test")
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_table_with_a_non_unique_index(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("a", types::varchar(10));
                    t.add_column("id", types::primary());
                    t.add_index("test", types::index(vec!["a"]));
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model User {
            a String
            id      Int @id @default(autoincrement())
            @@index([a], name: "test")
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_table_with_a_multi_column_non_unique_index(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("a", types::varchar(10));
                    t.add_column("b", types::varchar(10));
                    t.add_column("id", types::primary());
                    t.add_index("test", types::index(vec!["a", "b"]));
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! { r##"
        model User {
            a  String
            b  String
            id Int @id @default(autoincrement())
            @@index([a,b], name: "test")
        }
    "##};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

// SQLite does not have a serial type that's not a primary key.
#[test_each_connector(ignore("sqlite"))]
async fn a_table_with_non_id_autoincrement(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("Test", |t| {
                    t.add_column("id", types::integer().primary(true));
                    t.add_column("authorId", types::serial().unique(true));
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r#"
        model Test {
            id       Int @id
            authorId Int @default(autoincrement()) @unique
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn default_values(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("Test", |t| {
                    t.add_column("id", types::primary());
                    t.add_column(
                        "string_static_char",
                        types::custom("char(5)").default("test").nullable(true),
                    );
                    t.add_column(
                        "string_static_char_null",
                        types::r#char(5).default(types::null()).nullable(true),
                    );
                    t.add_column(
                        "string_static_varchar",
                        types::varchar(5).default("test").nullable(true),
                    );
                    t.add_column("int_static", types::integer().default(2).nullable(true));
                    t.add_column("float_static", types::float().default(1.43).nullable(true));
                    t.add_column("boolean_static", types::boolean().default(true).nullable(true));
                    t.add_column(
                        "datetime_now",
                        types::datetime().default(functions::current_timestamp()).nullable(true),
                    );
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! { r#"
        model Test {
            id                      Int       @id @default(autoincrement())
            string_static_char      String?   @default("test")
            string_static_char_null String?
            string_static_varchar   String?   @default("test")
            int_static              Int?      @default(2)
            float_static            Float?    @default(1.43)
            boolean_static          Boolean?  @default(true)
            datetime_now            DateTime? @default(now())
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn pg_default_value_as_dbgenerated(api: &TestApi) -> crate::TestResult {
    let sequence = "CREATE SEQUENCE test_seq START 1".to_string();
    api.database().execute_raw(&sequence, &[]).await?;

    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("string_function text Default 'Concatenated'||E'\n'");
                t.inject_custom("int_serial Serial4");
                t.inject_custom("int_function Integer DEFAULT EXTRACT(year from TIMESTAMP '2001-02-16 20:38:40')");
                t.inject_custom("int_sequence Integer DEFAULT nextval('test_seq')"); // todo this is not recognized as autoincrement
                t.inject_custom("datetime_now TIMESTAMP DEFAULT NOW()");
                t.inject_custom("datetime_now_lc TIMESTAMP DEFAULT now()");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Test {
            id                      Int       @id @default(autoincrement())
            string_function         String?   @default(dbgenerated())
            int_serial              Int       @default(autoincrement())
            int_function            Int?      @default(dbgenerated())
            int_sequence            Int?      @default(dbgenerated())
            datetime_now            DateTime? @default(now())
            datetime_now_lc         DateTime? @default(now())
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

//todo maybe need to split due to
// no function default values on mysql 5.7 and 8.0 -.-
// maria db allows this
#[test_each_connector(tags("mysql"))]
async fn my_default_value_as_dbgenerated(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("datetime_now TIMESTAMP NULL DEFAULT CURRENT_TIMESTAMP");
                t.inject_custom("datetime_now_lc TIMESTAMP NULL DEFAULT current_timestamp");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model Test {
            id                      Int                 @id @default(autoincrement())
            datetime_now            DateTime?           @default(now())
            datetime_now_lc         DateTime?           @default(now())
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("mysql_8"))]
async fn a_table_with_an_index_that_contains_expressions_should_be_ignored(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("Test", |t| {
                    t.add_column("id", types::integer().primary(true));
                    t.add_column("parentId", types::integer().nullable(true));
                    t.add_column("name", types::varchar(45).nullable(true));
                    t.inject_custom("UNIQUE KEY `SampleTableUniqueIndexName` (`name`,(ifnull(`parentId`,-(1))))");
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r#"
        model Test {
            id       Int     @id
            parentId Int?
            name     String?
        }      
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn default_values_on_lists_should_be_ignored(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("ints integer[] DEFAULT array[]::integer[]");
                t.inject_custom("ints2 integer[] DEFAULT '{}'");
            });
        })
        .await?;

    let dm = indoc! {r#"
        datasource pg {
            provider = "postgres"
            url = "postgresql://localhost:5432"
        }

        model User {
            id      Int @id @default(autoincrement())
            ints    Int[]
            ints2   Int[]
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

    assert_eq_datamodels!(dm, &result);

    Ok(())
}

// MySQL doesn't have partial indices.
#[test_each_connector(ignore("mysql"))]
async fn a_table_with_partial_indexes_should_ignore_them(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("pages", move |t| {
                t.add_column("id", types::primary());
                t.add_column("staticId", types::integer().nullable(false));
                t.add_column("latest", types::integer().nullable(false));
                t.add_column("other", types::integer().nullable(false));
                t.add_index("full", types::index(vec!["other"]).unique(true));
                t.add_partial_index("partial", types::index(vec!["staticId"]).unique(true), "latest = 1");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model pages {
            id       Int     @id @default(autoincrement())
            staticId Int
            latest   Int
            other    Int     @unique
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn a_table_with_not_null_partial_index_should_not_be_ignored(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("pages", move |t| {
                t.add_column("id", types::primary());
                t.add_column("static", types::integer().nullable(false));

                t.add_partial_index(
                    "partial",
                    types::index(vec!["static"]).unique(true),
                    "static IS NOT NULL",
                );
            });
        })
        .await?;

    let dm = indoc! {r#"
        model pages {
            id     Int     @id @default(autoincrement())
            static Int     @unique
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn a_table_with_two_not_null_partial_indices_in_conjunct_should_not_be_ignored(
    api: &TestApi,
) -> crate::TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("pages", move |t| {
                t.add_column("id", types::primary());
                t.add_column("static", types::integer().nullable(false));
                t.add_column("other", types::integer().nullable(false));

                t.add_partial_index(
                    "partial",
                    types::index(vec!["static", "other"]).unique(true),
                    "static IS NOT NULL    and other IS not NULL",
                );
            });
        })
        .await?;

    let dm = indoc! {r#"
        model pages {
            id     Int     @id @default(autoincrement())
            static Int
            other  Int
            @@unique([static, other], name: "partial")
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn a_table_with_two_partial_indices_in_conjunct_should_be_ignored(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("pages", move |t| {
                t.add_column("id", types::primary());
                t.add_column("static", types::integer().nullable(false));
                t.add_column("other", types::integer().nullable(false));

                t.add_partial_index(
                    "partial",
                    types::index(vec!["static", "other"]).unique(true),
                    "static IS NOT NULL AND other > 100",
                );
            });
        })
        .await?;

    let dm = indoc! {r#"
        model pages {
            id     Int     @id @default(autoincrement())
            static Int
            other  Int
        }
    "#};

    assert_eq_datamodels!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_json_type_must_work(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("json", types::json());
            });
        })
        .await?;

    let dm = indoc! {r#"
        datasource postgres {
            provider = "postgres"
            url = "postgresql://asdlj"
        }

        model Blog {
            id      Int @id @default(autoincrement())
            json    Json
        }
    "#};

    let expected = format!(
        r#"
        datasource postgres {{
            provider = "postgres"
            url = "postgresql://asdlj"
        }}

        {}
    "#,
        api.introspect().await?
    );

    assert_eq_datamodels!(dm, &expected);

    Ok(())
}
