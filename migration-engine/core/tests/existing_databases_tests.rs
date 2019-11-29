#![allow(unused)]
mod test_harness;
use barrel::{types, Migration, SqlVariant};
use migration_core::api::GenericApi;
use pretty_assertions::{assert_eq, assert_ne};
use sql_connection::SyncSqlConnection;
use quaint::prelude::SqlFamily;
use sql_migration_connector::SqlMigrationConnector;
use sql_schema_describer::*;
use std::sync::Arc;

use test_harness::*;

#[test_each_connector]
async fn adding_a_model_for_an_existing_table_must_work(api: &TestApi) {
    let initial_result = api.barrel().execute(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("id", types::primary());
        });
    });

    let dm = r#"
            model Blog {
                id Int @id
            }
        "#;
    let result = api.infer_and_apply(&dm).await.sql_schema;

    assert_eq!(initial_result, result);
}

#[test]
fn bigint_columns_must_work() {
    // TODO: port when barrel supports arbitray primary keys
}

#[test_each_connector]
async fn removing_a_model_for_a_table_that_is_already_deleted_must_work(api: &TestApi) {
    let dm1 = r#"
            model Blog {
                id Int @id
            }

            model Post {
                id Int @id
            }
        "#;
    let initial_result = api.infer_and_apply(&dm1).await.sql_schema;
    assert!(initial_result.has_table("Post"));

    let result = api.barrel().execute(|migration| {
        migration.drop_table("Post");
    });

    assert!(!result.has_table("Post"));

    let dm2 = r#"
            model Blog {
                id Int @id
            }
        "#;
    let final_result = api.infer_and_apply(&dm2).await.sql_schema;
    assert_eq!(result, final_result);
}

#[test_each_connector]
async fn creating_a_field_for_an_existing_column_with_a_compatible_type_must_work(api: &TestApi) {
    let initial_result = api.barrel().execute(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("id", types::primary());
            t.add_column("title", types::text());
        });
    });
    let dm = r#"
            model Blog {
                id Int @id
                title String
            }
        "#;
    let result = api.infer_and_apply(&dm).await.sql_schema;
    assert_eq!(initial_result, result);
}

#[test_each_connector]
async fn creating_a_field_for_an_existing_column_and_changing_its_type_must_work(api: &TestApi) {
    let initial_result = api.barrel().execute(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("id", types::primary());
            t.add_column("title", types::integer().nullable(true));
        });
    });
    let initial_column = initial_result.table_bang("Blog").column_bang("title");
    assert_eq!(initial_column.tpe.family, ColumnTypeFamily::Int);
    assert_eq!(initial_column.is_required(), false);

    let dm = r#"
            model Blog {
                id Int @id
                title String @unique
            }
        "#;
    let result = api.infer_and_apply(&dm).await.sql_schema;
    let table = result.table_bang("Blog");
    let column = table.column_bang("title");
    assert_eq!(column.tpe.family, ColumnTypeFamily::String);
    assert_eq!(column.is_required(), true);
    let index = table.indices.iter().find(|i| i.columns == vec!["title"]);
    assert_eq!(index.is_some(), true);
    assert_eq!(index.unwrap().tpe, IndexType::Unique);
}

#[test_each_connector]
async fn creating_a_field_for_an_existing_column_and_simultaneously_making_it_optional(api: &TestApi) {
    let initial_result = api.barrel().execute(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("id", types::primary());
            t.add_column("title", types::text());
        });
    });
    let initial_column = initial_result.table_bang("Blog").column_bang("title");
    assert_eq!(initial_column.is_required(), true);

    let dm = r#"
            model Blog {
                id Int @id
                title String?
            }
        "#;
    let result = api.infer_and_apply(&dm).await.sql_schema;
    let column = result.table_bang("Blog").column_bang("title");
    assert_eq!(column.is_required(), false);
}

#[test_one_connector(connector = "postgres")]
async fn creating_a_scalar_list_field_for_an_existing_table_must_work(api: &TestApi) {
    let dm1 = r#"
            model Blog {
                id Int @id
            }
        "#;
    let initial_result = api.infer_and_apply(&dm1).await.sql_schema;
    assert!(!initial_result.has_table("Blog_tags"));

    let mut result = api.barrel().execute(|migration| {
        migration.change_table("Blog", |t| {
            let inner = types::text();
            t.add_column("tags", types::array(&inner));
        });
    });

    let dm2 = r#"
            model Blog {
                id Int @id
                tags String[]
            }
        "#;
    let mut final_result = api.infer_and_apply(&dm2).await.sql_schema;
    assert_eq!(result, final_result);
}

#[test_each_connector]
async fn delete_a_field_for_a_non_existent_column_must_work(api: &TestApi) {
    let dm1 = r#"
            model Blog {
                id Int @id
                title String
            }
        "#;
    let initial_result = api.infer_and_apply(&dm1).await.sql_schema;
    assert_eq!(initial_result.table_bang("Blog").column("title").is_some(), true);

    let result = api.barrel().execute(|migration| {
        // sqlite does not support dropping columns. So we are emulating it..
        migration.drop_table("Blog");
        migration.create_table("Blog", |t| {
            t.add_column("id", types::primary());
        });
    });
    assert_eq!(result.table_bang("Blog").column("title").is_some(), false);

    let dm2 = r#"
            model Blog {
                id Int @id
            }
        "#;
    let final_result = api.infer_and_apply(&dm2).await.sql_schema;
    assert_eq!(result, final_result);
}

#[test_each_connector]
async fn deleting_a_scalar_list_field_for_a_non_existent_list_table_must_work(api: &TestApi) {
    let dm1 = r#"
            model Blog {
                id Int @id
                tags String[]
            }
        "#;
    let initial_result = api.infer_and_apply(&dm1).await.sql_schema;
    assert!(initial_result.has_table("Blog_tags"));

    let result = api.barrel().execute(|migration| {
        migration.drop_table("Blog_tags");
    });
    assert!(!result.has_table("Blog_tags"));

    let dm2 = r#"
            model Blog {
                id Int @id
            }
        "#;
    let final_result = api.infer_and_apply(&dm2).await.sql_schema;
    assert_eq!(result, final_result);
}

#[test_each_connector]
async fn updating_a_field_for_a_non_existent_column(api: &TestApi) {
    let dm1 = r#"
            model Blog {
                id Int @id
                title String
            }
        "#;
    let initial_result = api.infer_and_apply(&dm1).await.sql_schema;
    let initial_column = initial_result.table_bang("Blog").column_bang("title");
    assert_eq!(initial_column.tpe.family, ColumnTypeFamily::String);

    let result = api.barrel().execute(|migration| {
        // sqlite does not support dropping columns. So we are emulating it..
        migration.drop_table("Blog");
        migration.create_table("Blog", |t| {
            t.add_column("id", types::primary());
        });
    });
    assert_eq!(result.table_bang("Blog").column("title").is_some(), false);

    let dm2 = r#"
            model Blog {
                id Int @id
                title Int @unique
            }
        "#;
    let final_result = api.infer_and_apply(&dm2).await.sql_schema;
    let final_column = final_result.table_bang("Blog").column_bang("title");
    assert_eq!(final_column.tpe.family, ColumnTypeFamily::Int);
    let index = final_result
        .table_bang("Blog")
        .indices
        .iter()
        .find(|i| i.columns == vec!["title"]);
    assert_eq!(index.is_some(), true);
    assert_eq!(index.unwrap().tpe, IndexType::Unique);
}

#[test_each_connector]
async fn renaming_a_field_where_the_column_was_already_renamed_must_work(api: &TestApi) {
    let dm1 = r#"
            model Blog {
                id Int @id
                title String
            }
        "#;
    let initial_result = api.infer_and_apply(&dm1).await.sql_schema;
    let initial_column = initial_result.table_bang("Blog").column_bang("title");
    assert_eq!(initial_column.tpe.family, ColumnTypeFamily::String);

    let result = api.barrel().execute(|migration| {
        // sqlite does not support renaming columns. So we are emulating it..
        migration.drop_table("Blog");
        migration.create_table("Blog", |t| {
            t.add_column("id", types::primary());
            t.add_column("new_title", types::text());
        });
    });
    assert_eq!(result.table_bang("Blog").column("new_title").is_some(), true);

    let dm2 = r#"
            model Blog {
                id Int @id
                title Float @map(name: "new_title")
            }
        "#;

    let final_result = api.infer_and_apply(&dm2).await.sql_schema;

    let final_column = final_result.table_bang("Blog").column_bang("new_title");

    assert_eq!(final_column.tpe.family, ColumnTypeFamily::Float);
    assert_eq!(final_result.table_bang("Blog").column("title").is_some(), false);
}
