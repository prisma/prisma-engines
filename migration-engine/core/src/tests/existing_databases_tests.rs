use super::test_harness::*;
use crate::commands::{
    ApplyMigrationCommand, ApplyMigrationInput, InferMigrationStepsCommand, InferMigrationStepsInput,
};
use barrel::types;
use pretty_assertions::assert_eq;
use quaint::prelude::SqlFamily;
use sql_schema_describer::*;

#[test_each_connector]
async fn adding_a_model_for_an_existing_table_must_work(api: &TestApi) {
    let initial_result = api
        .barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

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

    let result = api
        .barrel()
        .execute(|migration| {
            migration.drop_table("Post");
        })
        .await;

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
    let is_mysql = api.is_mysql();
    let initial_result = api
        .barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
                // We add a default because the migration engine always adds defaults to facilitate migration of required columns.
                t.add_column(
                    "title",
                    if is_mysql {
                        types::varchar(181).default("")
                    } else {
                        types::text().default("")
                    },
                );
            });
        })
        .await;
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
    let initial_result = api
        .barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("title", types::integer().nullable(true));
            });
        })
        .await;
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
    assert!(column.is_required());
    let index = table.indices.iter().find(|i| i.columns == &["title"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);
}

#[test_each_connector]
async fn creating_a_field_for_an_existing_column_and_simultaneously_making_it_optional(api: &TestApi) {
    let initial_result = api
        .barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("title", types::text());
            });
        })
        .await;
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
            datasource pg {
              provider = "postgres"
              url = "postgres://localhost:5432"
            }

            model Blog {
                id Int @id
            }
        "#;
    let initial_result = api.infer_and_apply(&dm1).await.sql_schema;
    assert!(!initial_result.has_table("Blog_tags"));

    let result = api
        .barrel()
        .execute(|migration| {
            migration.change_table("Blog", |t| {
                let inner = types::text();
                t.add_column("tags", types::array(&inner));
            });
        })
        .await;

    let dm2 = r#"
            datasource pg {
              provider = "postgres"
              url = "postgres://localhost:5432"
            }

            model Blog {
                id Int @id
                tags String[]
            }
        "#;
    let final_result = api.infer_and_apply(&dm2).await.sql_schema;
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

    let result = api
        .barrel()
        .execute(|migration| {
            // sqlite does not support dropping columns. So we are emulating it..
            migration.drop_table("Blog");
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;
    assert_eq!(result.table_bang("Blog").column("title").is_some(), false);

    let dm2 = r#"
            model Blog {
                id Int @id
            }
        "#;
    let final_result = api.infer_and_apply(&dm2).await.sql_schema;
    assert_eq!(result, final_result);
}

#[test_one_connector(connector = "postgres")]
async fn deleting_a_scalar_list_field_for_a_non_existent_column_must_work(api: &TestApi) {
    let dm1 = r#"
            datasource pg {
              provider = "postgres"
              url = "postgres://localhost:5432"
            }

            model Blog {
                id Int @id
                tags String[]
            }
        "#;

    let _ = api.infer_and_apply(&dm1).await.sql_schema;

    let result = api
        .barrel()
        .execute(|migration| {
            // sqlite does not support dropping columns. So we are emulating it..
            migration.drop_table("Blog");
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let dm2 = r#"
            datasource pg {
              provider = "postgres"
              url = "postgres://localhost:5432"
            }

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

    let result = api
        .barrel()
        .execute(|migration| {
            // sqlite does not support dropping columns. So we are emulating it..
            migration.drop_table("Blog");
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;
    assert!(result.table_bang("Blog").column("title").is_none());

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

    let result = api
        .barrel()
        .execute(|migration| {
            // sqlite does not support renaming columns. So we are emulating it..
            migration.drop_table("Blog");
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("new_title", types::text());
            });
        })
        .await;
    assert!(result.table_bang("Blog").column("new_title").is_some());

    let dm2 = r#"
            model Blog {
                id Int @id
                title Float @map(name: "new_title")
            }
        "#;

    let final_result = api.infer_and_apply(&dm2).await.sql_schema;

    let final_column = final_result.table_bang("Blog").column_bang("new_title");

    assert_eq!(final_column.tpe.family, ColumnTypeFamily::Float);
    assert!(final_result.table_bang("Blog").column("title").is_none());
}

#[test_each_connector]
async fn removing_a_default_from_a_non_nullable_foreign_key_column_must_warn(api: &TestApi) {
    let sql_family = api.sql_family();
    let sql_schema = api
        .barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
                // Barrel fails to create foreign key columns with defaults (bad SQL).
                let fk = match sql_family {
                    SqlFamily::Postgres => r#""user" INTEGER DEFAULT 1, FOREIGN KEY ("user") REFERENCES "User" ("id")"#,
                    _ => "user INTEGER DEFAULT 1, FOREIGN KEY (user) REFERENCES User (id)",
                };

                t.inject_custom(fk);
            });
        })
        .await;

    assert!(sql_schema.table_bang("Blog").column("user").unwrap().default.is_some());

    let dm = r#"
        model User {
            id Int @id
        }

        model Blog {
            id Int @id
            user User
        }
    "#;

    let infer_input = InferMigrationStepsInput {
        datamodel: dm.into(),
        assume_to_be_applied: Some(Vec::new()),
        assume_applied_migrations: None,
        migration_id: "test-migration".into(),
    };

    let result = api
        .execute_command::<InferMigrationStepsCommand>(&infer_input)
        .await
        .unwrap();

    let apply_input = ApplyMigrationInput {
        steps: result.datamodel_steps,
        force: Some(false),
        migration_id: "test-migration".into(),
    };

    let result = api
        .execute_command::<ApplyMigrationCommand>(&apply_input)
        .await
        .unwrap();

    let expected_warning = "The migration is about to remove a default value on the foreign key field `Blog.user`.";
    assert_eq!(
        result.warnings,
        &[migration_connector::MigrationWarning {
            description: expected_warning.into()
        }]
    );
}
