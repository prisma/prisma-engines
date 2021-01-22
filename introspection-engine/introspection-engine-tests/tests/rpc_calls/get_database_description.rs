use barrel::types;
use introspection_engine_tests::{assert_eq_schema, test_api::*, BarrelMigrationExecutor};
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

    let expected = r#"
    SqlSchema {
        tables: [
            Table {
                name: "Blog",
            columns: [
                 Column {
                    name: "id",
                    tpe: ColumnType {
                        full_data_type: "int(11)",
                        family: Int,
                        arity: Required,
                        native_type: Some(
                                String(
                                    "Int",
                            ),
                        ),
                    },
                    default: None,
                    auto_increment: true,
                },
                Column {
                    name: "string",
                    tpe: ColumnType {
                        full_data_type: "text",
                        family: String,
                        arity: Required,
                        native_type: Some(
                                String(
                                    "Text",
                            ),
                        ),
                    },
                    default: None,
                    auto_increment: false,
                },
            ],
            indices: [],
            primary_key: Some(
                    PrimaryKey {
                        columns: [
                            "id",
                    ],
                    sequence: None,
                    constraint_name: None,
                },
            ),
            foreign_keys: [],
        },
    ],
    enums: [],
    sequences: [],
}"#;

    assert_eq_schema!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("mysql_8"))]
async fn database_description_for_mysql_8_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = r#"
    SqlSchema {
        tables: [
            Table {
                name: "Blog",
            columns: [
                 Column {
                    name: "id",
                    tpe: ColumnType {
                        full_data_type: "int",
                        family: Int,
                        arity: Required,
                        native_type: Some(
                                String(
                                    "Int",
                            ),
                        ),
                    },
                    default: None,
                    auto_increment: true,
                },
                Column {
                    name: "string",
                    tpe: ColumnType {
                        full_data_type: "text",
                        family: String,
                        arity: Required,
                        native_type: Some(
                                String(
                                    "Text",
                            ),
                        ),
                    },
                    default: None,
                    auto_increment: false,
                },
            ],
            indices: [],
            primary_key: Some(
                    PrimaryKey {
                        columns: [
                            "id",
                    ],
                    sequence: None,
                    constraint_name: None,
                },
            ),
            foreign_keys: [],
        },
    ],
    enums: [],
    sequences: [],
}"#;

    assert_eq_schema!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn database_description_for_postgres_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = r#"
    SqlSchema {
        tables: [
            Table {
                name: "Blog",
            columns: [
               Column {
                    name: "id",
                    tpe: ColumnType {
                        full_data_type: "int4",
                        family: Int,
                        arity: Required,
                        native_type: Some(
                                String(
                                    "Integer",
                            ),
                        ),
                    },
                    default: Some(
                            DefaultValue {
                                kind: SEQUENCE(
                                    "Blog_id_seq",
                            ),
                            constraint_name: None,
                        },
                    ),
                    auto_increment: true,
                },
                Column {
                    name: "string",
                    tpe: ColumnType {
                        full_data_type: "text",
                        family: String,
                        arity: Required,
                        native_type: Some(
                                String(
                                    "Text",
                            ),
                        ),
                    },
                    default: None,
                    auto_increment: false,
                },
            ],
            indices: [],
            primary_key: Some(
                    PrimaryKey {
                        columns: [
                            "id",
                    ],
                    sequence: Some(
                            Sequence {
                                name: "Blog_id_seq",
                        },
                    ),
                    constraint_name: Some(
                            "Blog_pkey",
                    ),
                },
            ),
            foreign_keys: [],
        },
    ],
    enums: [],
    sequences: [
            Sequence {
                name: "Blog_id_seq",
        },
    ],
}"#;

    assert_eq_schema!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn database_description_for_sqlite_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = r#"
    SqlSchema {
        tables: [
            Table {
                name: "Blog",
            columns: [
                 Column {
                    name: "id",
                    tpe: ColumnType {
                        full_data_type: "INTEGER",
                        family: Int,
                        arity: Required,
                        native_type: None,
                    },
                    default: None,
                    auto_increment: true,
                },
                Column {
                    name: "string",
                    tpe: ColumnType {
                        full_data_type: "TEXT",
                        family: String,
                        arity: Required,
                        native_type: None,
                    },
                    default: None,
                    auto_increment: false,
                },
            ],
            indices: [],
            primary_key: Some(
                    PrimaryKey {
                        columns: [
                            "id",
                    ],
                    sequence: None,
                    constraint_name: None,
                },
            ),
            foreign_keys: [],
        },
    ],
    enums: [],
    sequences: [],
}"#;

    assert_eq_schema!(expected, api.get_database_description().await?);

    Ok(())
}

//cant assert the string since the PK constraint name is random
//just checking it does not error
#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn database_description_for_mssql_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    api.get_database_description().await?;

    Ok(())
}
