use barrel::types;
use introspection_engine_tests::{assert_eq_json, test_api::*, BarrelMigrationExecutor};
use serde::Deserialize;
use serde_json::json;
use test_macros::test_each_connector;

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
                            "SEQUENCE": "Blog_id_seq"
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
                        "name": "Blog_id_seq"
                    },
                    "constraintName": "Blog_pkey"
                },
                "foreignKeys": []
            }
        ],
        "enums": [],
        "sequences": [
            {
                "name": "Blog_id_seq"
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

// MAY GOD FORGIVE ME! ->

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct Response {
    tables: Vec<Table>,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct Table {
    name: String,
    columns: Vec<Column>,
    primary_key: PrimaryKey,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct Column {
    name: String,
    tpe: ColumnType,
    default: Option<String>,
    auto_increment: bool,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct ColumnType {
    data_type: String,
    full_data_type: String,
    character_maximum_length: Option<u64>,
    family: String,
    arity: String,
    native_type: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PrimaryKey {
    columns: Vec<String>,
    sequence: Option<String>,
    constraint_name: Option<String>,
}

impl PartialEq for PrimaryKey {
    fn eq(&self, other: &Self) -> bool {
        let columns_match = self.columns == other.columns;
        let sequences_match = self.sequence == other.sequence;

        let constraints_match = match (self.constraint_name.as_ref(), other.constraint_name.as_ref()) {
            (None, None) => true,
            (Some(_), None) | (None, Some(_)) => false,
            (Some(left), Some(right)) => {
                let mut splitted = left.split("__");
                let start = format!("{}__{}__", splitted.next().unwrap(), splitted.next().unwrap());
                right.starts_with(&start)
            }
        };

        columns_match && sequences_match && constraints_match
    }
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn database_description_for_mssql_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = Response {
        tables: vec![Table {
            name: "Blog".into(),
            columns: vec![
                Column {
                    name: "id".into(),
                    tpe: ColumnType {
                        data_type: "int".into(),
                        full_data_type: "int".into(),
                        character_maximum_length: None,
                        family: "int".into(),
                        arity: "required".into(),
                        native_type: "Int".into(),
                    },
                    default: None,
                    auto_increment: true,
                },
                Column {
                    name: "string".into(),
                    tpe: ColumnType {
                        data_type: "text".into(),
                        full_data_type: "text".into(),
                        character_maximum_length: Some(2147483647),
                        family: "string".into(),
                        arity: "required".into(),
                        native_type: "Text".into(),
                    },
                    default: None,
                    auto_increment: false,
                },
            ],
            primary_key: PrimaryKey {
                columns: vec!["id".into()],
                sequence: None,
                constraint_name: Some("PK__Blog__*".into()),
            },
        }],
    };

    let result: Response = serde_json::from_str(&api.get_database_description().await?)?;

    assert_eq!(expected, result);

    Ok(())
}
