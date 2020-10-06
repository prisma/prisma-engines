use barrel::types;
use introspection_engine_tests::{assert_eq_json, test_api::*, BarrelMigrationExecutor};
use serde_json::json;
use test_macros::test_each_connector_mssql as test_each_connector;

async fn setup_blog(barrel: &BarrelMigrationExecutor) -> crate::TestResult {
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

#[test_each_connector(tags("mysql_5_6", "mariadb"))]
async fn database_description_for_mysql_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = json!({
        "tables": [
            {
                "name": "Blog",
                "columns": [
                    {
                        "name": "id",
                        "tpe": {
                            "dataType": "int",
                            "fullDataType": "int(11)",
                            "characterMaximumLength": null,
                            "family": "int",
                            "arity": "required",
                            "nativeType": "Int"
                        },
                        "default": null,
                        "autoIncrement": true
                    },
                    {
                        "name": "string",
                        "tpe": {
                            "dataType": "text",
                            "fullDataType": "text",
                            "characterMaximumLength": 65535,
                            "family": "string",
                            "arity": "required",
                            "nativeType": "Text"
                        },
                        "default": null,
                        "autoIncrement": false
                    }
                ],
                "indices": [],
                "primaryKey": {
                    "columns": [
                        "id"
                    ],
                    "sequence": null,
                    "constraintName": null
                },
                "foreignKeys": []
            }
        ],
        "enums": [],
        "sequences": []
    });

    assert_eq_json!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("mysql_8"))]
async fn database_description_for_mysql_8_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = json!({
        "tables": [
            {
                "name": "Blog",
                "columns": [
                    {
                        "name": "id",
                        "tpe": {
                            "dataType": "int",
                            "fullDataType": "int",
                            "characterMaximumLength": null,
                            "family": "int",
                            "arity": "required",
                            "nativeType": "Int"
                        },
                        "default": null,
                        "autoIncrement": true
                    },
                    {
                        "name": "string",
                        "tpe": {
                            "dataType": "text",
                            "fullDataType": "text",
                            "characterMaximumLength": 65535,
                            "family": "string",
                            "arity": "required",
                            "nativeType": "Text"
                        },
                        "default": null,
                        "autoIncrement": false
                    }
                ],
                "indices": [],
                "primaryKey": {
                    "columns": [
                        "id"
                    ],
                    "sequence": null,
                    "constraintName": null
                },
                "foreignKeys": []
            }
        ],
        "enums": [],
        "sequences": []
    });

    assert_eq_json!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn database_description_for_postgres_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = json!({
        "tables": [
            {
                "name": "Blog",
                "columns": [
                    {
                        "name": "id",
                        "tpe": {
                            "dataType": "integer",
                            "fullDataType": "int4",
                            "characterMaximumLength": null,
                            "family": "int",
                            "arity": "required",
                            "nativeType": "Integer"
                        },
                        "default": {
                            "SEQUENCE": "nextval('\"Blog_id_seq\"'::regclass)"
                        },
                        "autoIncrement": true
                    },
                    {
                        "name": "string",
                        "tpe": {
                            "dataType": "text",
                            "fullDataType": "text",
                            "characterMaximumLength": null,
                            "family": "string",
                            "arity": "required",
                            "nativeType": "Text"
                        },
                        "default": null,
                        "autoIncrement": false
                    }
                ],
                "indices": [],
                "primaryKey": {
                    "columns": [
                        "id"
                    ],
                    "sequence": {
                        "name": "Blog_id_seq",
                        "initialValue": 1,
                        "allocationSize": 1
                    },
                    "constraintName": "Blog_pkey"
                },
                "foreignKeys": []
            }
        ],
        "enums": [],
        "sequences": [
            {
                "name": "Blog_id_seq",
                "initialValue": 1,
                "allocationSize": 1
            }
        ]
    });

    assert_eq_json!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn database_description_for_sqlite_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = json!({
        "tables": [
            {
                "name": "Blog",
                "columns": [
                    {
                        "name": "id",
                        "tpe": {
                            "dataType": "INTEGER",
                            "fullDataType": "INTEGER",
                            "characterMaximumLength": null,
                            "family": "int",
                            "arity": "required",
                            "nativeType": null
                        },
                        "default": null,
                        "autoIncrement": true
                    },
                    {
                        "name": "string",
                        "tpe": {
                            "dataType": "TEXT",
                            "fullDataType": "TEXT",
                            "characterMaximumLength": null,
                            "family": "string",
                            "arity": "required",
                            "nativeType": null
                        },
                        "default": null,
                        "autoIncrement": false
                    }
                ],
                "indices": [],
                "primaryKey": {
                    "columns": [
                        "id"
                    ],
                    "sequence": null,
                    "constraintName": null
                },
                "foreignKeys": []
            }
        ],
        "enums": [],
        "sequences": []
    });

    assert_eq_json!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn database_description_for_mssql_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = json!({
        "tables": [
            {
                "name": "Blog",
                "columns": [
                    {
                        "name": "id",
                        "tpe": {
                            "dataType": "int",
                            "fullDataType": "int",
                            "characterMaximumLength": null,
                            "family": "int",
                            "arity": "required",
                            "nativeType": null
                        },
                        "default": null,
                        "autoIncrement": true
                    },
                    {
                        "name": "string",
                        "tpe": {
                            "dataType": "text",
                            "fullDataType": "text",
                            "characterMaximumLength": 2147483647,
                            "family": "string",
                            "arity": "required",
                            "nativeType": null
                        },
                        "default": null,
                        "autoIncrement": false
                    }
                ],
                "indices": [],
                "primaryKey": {
                    "columns": [
                        "id"
                    ],
                    "sequence": null,
                    "constraintName": null
                },
                "foreignKeys": []
            }
        ],
        "enums": [],
        "sequences": []
    });

    assert_eq_json!(expected, api.get_database_description().await?);

    Ok(())
}
