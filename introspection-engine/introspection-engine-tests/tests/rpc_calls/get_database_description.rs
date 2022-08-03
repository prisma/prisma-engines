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
              "name": "Blog"
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
          "foreign_key_columns": [],
          "indexes": [
            {
              "table_id": 0,
              "index_name": "",
              "tpe": "PrimaryKey"
            }
          ],
          "index_columns": [
            {
              "index_id": 0,
              "column_id": 0,
              "sort_order": "Asc",
              "length": null
            }
          ],
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
          "namespaces": [],
          "default_namespace": 0,
          "tables": [
            {
              "name": "Blog",
              "namespace": 0
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
          "foreign_key_columns": [],
          "indexes": [
            {
              "table_id": 0,
              "index_name": "",
              "tpe": "PrimaryKey"
            }
          ],
          "index_columns": [
            {
              "index_id": 0,
              "column_id": 0,
              "sort_order": "Asc",
              "length": null
            }
          ],
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
          "namespaces": [],
          "tables": [
            {
              "namespace": 0,
              "name": "Blog"
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
          "foreign_key_columns": [],
          "indexes": [
            {
              "table_id": 0,
              "index_name": "Blog_pkey",
              "tpe": "PrimaryKey"
            }
          ],
          "index_columns": [
            {
              "index_id": 0,
              "column_id": 0,
              "sort_order": "Asc",
              "length": null
            }
          ],
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
          "namespaces": [],
          "default_namespace": 0,
          "tables": [
            {
              "name": "Blog",
              "namespace": 0
            }
          ],
          "enums": [],
          "columns": [
            [
              0,
              {
                "name": "id",
                "tpe": {
                  "full_data_type": "integer",
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
                  "full_data_type": "text",
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
          "foreign_key_columns": [],
          "indexes": [
            {
              "table_id": 0,
              "index_name": "",
              "tpe": "PrimaryKey"
            }
          ],
          "index_columns": [
            {
              "index_id": 0,
              "column_id": 0,
              "sort_order": null,
              "length": null
            }
          ],
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
