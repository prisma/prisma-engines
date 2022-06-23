use barrel::types;
use expect_test::expect;
use introspection_engine_tests::{test_api::*, BarrelMigrationExecutor};
use test_macros::test_connector;

async fn setup_blog(barrel: &BarrelMigrationExecutor) -> TestResult {
    barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("string", types::text());
            });
        })
        .await?;

    Ok(())
}

#[test_connector(tags(Mysql56, Mariadb))]
async fn database_description_for_mysql_should_work(api: &TestApi) -> TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = expect![[r#"
        {
          "tables": [
            {
              "name": "Blog",
              "indices": [],
              "primary_key": {
                "columns": [
                  {
                    "name": "id",
                    "length": null,
                    "sort_order": null
                  }
                ],
                "constraint_name": null
              }
            }
          ],
          "enums": [],
          "columns": [
            [
              0,
              {
                "name": "id",
                "tpe": {
                  "full_data_type": "int(11)",
                  "family": "Int",
                  "arity": "Required",
                  "native_type": "Int"
                },
                "default": null,
                "auto_increment": true
              }
            ],
            [
              0,
              {
                "name": "string",
                "tpe": {
                  "full_data_type": "text",
                  "family": "String",
                  "arity": "Required",
                  "native_type": "Text"
                },
                "default": null,
                "auto_increment": false
              }
            ]
          ],
          "foreign_keys": [],
          "views": [],
          "procedures": [],
          "user_defined_types": [],
          "connector_data": null
        }"#]];

    expected.assert_eq(&api.get_database_description().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn database_description_for_mysql_8_should_work(api: &TestApi) -> TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = expect![[r#"
        {
          "tables": [
            {
              "name": "Blog",
              "indices": [],
              "primary_key": {
                "columns": [
                  {
                    "name": "id",
                    "length": null,
                    "sort_order": null
                  }
                ],
                "constraint_name": null
              }
            }
          ],
          "enums": [],
          "columns": [
            [
              0,
              {
                "name": "id",
                "tpe": {
                  "full_data_type": "int",
                  "family": "Int",
                  "arity": "Required",
                  "native_type": "Int"
                },
                "default": null,
                "auto_increment": true
              }
            ],
            [
              0,
              {
                "name": "string",
                "tpe": {
                  "full_data_type": "text",
                  "family": "String",
                  "arity": "Required",
                  "native_type": "Text"
                },
                "default": null,
                "auto_increment": false
              }
            ]
          ],
          "foreign_keys": [],
          "views": [],
          "procedures": [],
          "user_defined_types": [],
          "connector_data": null
        }"#]];

    expected.assert_eq(&api.get_database_description().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn database_description_for_postgres_should_work(api: &TestApi) -> TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = expect![[r#"
        {
          "tables": [
            {
              "name": "Blog",
              "indices": [],
              "primary_key": {
                "columns": [
                  {
                    "name": "id",
                    "length": null,
                    "sort_order": null
                  }
                ],
                "constraint_name": "Blog_pkey"
              }
            }
          ],
          "enums": [],
          "columns": [
            [
              0,
              {
                "name": "id",
                "tpe": {
                  "full_data_type": "int4",
                  "family": "Int",
                  "arity": "Required",
                  "native_type": "Integer"
                },
                "default": {
                  "kind": {
                    "Sequence": "Blog_id_seq"
                  },
                  "constraint_name": null
                },
                "auto_increment": true
              }
            ],
            [
              0,
              {
                "name": "string",
                "tpe": {
                  "full_data_type": "text",
                  "family": "String",
                  "arity": "Required",
                  "native_type": "Text"
                },
                "default": null,
                "auto_increment": false
              }
            ]
          ],
          "foreign_keys": [],
          "views": [],
          "procedures": [],
          "user_defined_types": [],
          "connector_data": null
        }"#]];

    expected.assert_eq(&api.get_database_description().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn database_description_for_sqlite_should_work(api: &TestApi) -> TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = expect![[r#"
        {
          "tables": [
            {
              "name": "Blog",
              "indices": [],
              "primary_key": {
                "columns": [
                  {
                    "name": "id",
                    "length": null,
                    "sort_order": null
                  }
                ],
                "constraint_name": null
              }
            }
          ],
          "enums": [],
          "columns": [
            [
              0,
              {
                "name": "id",
                "tpe": {
                  "full_data_type": "INTEGER",
                  "family": "Int",
                  "arity": "Required",
                  "native_type": null
                },
                "default": null,
                "auto_increment": true
              }
            ],
            [
              0,
              {
                "name": "string",
                "tpe": {
                  "full_data_type": "TEXT",
                  "family": "String",
                  "arity": "Required",
                  "native_type": null
                },
                "default": null,
                "auto_increment": false
              }
            ]
          ],
          "foreign_keys": [],
          "views": [],
          "procedures": [],
          "user_defined_types": [],
          "connector_data": null
        }"#]];

    expected.assert_eq(&api.get_database_description().await?);

    Ok(())
}

//cant assert the string since the PK constraint name is random
//just checking it does not error
#[test_connector(tags(Mssql))]
async fn database_description_for_mssql_should_work(api: &TestApi) -> TestResult {
    setup_blog(&api.barrel()).await?;

    api.get_database_description().await?;

    Ok(())
}
