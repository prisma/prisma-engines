use barrel::types;
use migration_engine_tests::sql::*;
use pretty_assertions::assert_eq;
use quaint::prelude::Queryable;
use sql_schema_describer::*;

#[test_each_connector]
async fn adding_a_model_for_an_existing_table_must_work(api: &TestApi) -> TestResult {
    let initial_result = api
        .barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    let dm = r#"
        model Blog {
            id Int @id @default(autoincrement())
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_equals(&initial_result)?;

    Ok(())
}

#[test_each_connector]
async fn removing_a_model_for_a_table_that_is_already_deleted_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Blog {
            id Int @id
        }

        model Post {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;
    let initial_result = api.describe_database().await?;

    assert!(initial_result.get_table("Post").is_some());

    let result = api
        .barrel()
        .execute(|migration| {
            migration.drop_table("Post");
        })
        .await?;

    assert!(!result.get_table("Post").is_some());

    let dm2 = r#"
        model Blog {
            id Int @id
        }
    "#;

    api.schema_push(dm2).send().await?;

    api.assert_schema().await?.assert_equals(&result)?;

    Ok(())
}

#[test_each_connector]
async fn creating_a_field_for_an_existing_column_with_a_compatible_type_must_work(api: &TestApi) -> TestResult {
    let is_mysql = api.is_mysql();
    let is_mssql = api.is_mssql();
    let initial_result = api
        .barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
                t.add_column(
                    "title",
                    if is_mysql {
                        types::varchar(191)
                    } else if is_mssql {
                        types::custom("NVARCHAR(1000)")
                    } else {
                        types::text()
                    },
                );
            });
        })
        .await?;

    let dm = r#"
        model Blog {
            id      Int @id @default(autoincrement())
            title   String
        }
    "#;

    api.schema_push(dm).force(true).send().await?.assert_green()?;

    let final_schema = api.describe_database().await?;

    assert_eq!(initial_result, final_schema);

    Ok(())
}

#[test_each_connector]
async fn creating_a_field_for_an_existing_column_and_changing_its_type_must_work(api: &TestApi) -> TestResult {
    let initial_result = api
        .barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("title", types::integer().nullable(true));
            });
        })
        .await?;

    let initial_column = initial_result.table_bang("Blog").column_bang("title");
    assert_eq!(initial_column.tpe.family, ColumnTypeFamily::Int);
    assert_eq!(initial_column.is_required(), false);

    let dm = r#"
            model Blog {
                id Int @id
                title String @unique
            }
        "#;

    api.schema_push(dm).force(true).send().await?;

    api.assert_schema().await?.assert_table("Blog", |table| {
        table
            .assert_column("title", |col| col.assert_type_is_string()?.assert_is_required())?
            .assert_index_on_columns(&["title"], |idx| idx.assert_is_unique())
    })?;

    Ok(())
}

#[test_each_connector]
async fn creating_a_field_for_an_existing_column_and_simultaneously_making_it_optional(api: &TestApi) -> TestResult {
    let initial_result = api
        .barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("title", types::text());
            });
        })
        .await?;
    let initial_column = initial_result.table_bang("Blog").column_bang("title");
    assert_eq!(initial_column.is_required(), true);

    let dm = r#"
        model Blog {
            id Int @id
            title String?
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    let result = api.describe_database().await?;

    let column = result.table_bang("Blog").column_bang("title");
    assert!(!column.is_required());

    Ok(())
}

#[test_each_connector(capabilities("scalar_lists"))]
async fn creating_a_scalar_list_field_for_an_existing_table_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        datasource pg {
            provider = "postgres"
            url = "postgres://localhost:5432"
        }

        model Blog {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let initial_result = api.describe_database().await?;

    assert!(!initial_result.get_table("Blog_tags").is_some());

    let result = api
        .barrel()
        .execute(|migration| {
            migration.change_table("Blog", |t| {
                let inner = types::text();
                t.add_column("tags", types::array(&inner));
            });
        })
        .await?;

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

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_equals(&result)?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn delete_a_field_for_a_non_existent_column_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
            model Blog {
                id      Int @id
                title   String
            }
        "#;

    api.schema_push(dm1).send().await?;

    let initial_result = api.describe_database().await?;
    assert!(initial_result.table_bang("Blog").column("title").is_some());

    let result = api
        .barrel()
        .execute(|migration| {
            // sqlite does not support dropping columns. So we are emulating it..
            migration.drop_table("Blog");
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await?;

    assert!(result.table_bang("Blog").column("title").is_none());

    let dm2 = r#"
        model Blog {
            id Int @id @default(autoincrement())
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_equals(&result)?;

    Ok(())
}

#[test_each_connector(tags("sql"), capabilities("scalar_lists"))]
async fn deleting_a_scalar_list_field_for_a_non_existent_column_must_work(api: &TestApi) -> TestResult {
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

    api.schema_push(dm1).send().await?.assert_green()?;

    let result = api
        .barrel()
        .execute(|migration| {
            migration.change_table("Blog", |t| {
                t.drop_column("tags");
            });
        })
        .await?;

    let dm2 = r#"
            datasource pg {
              provider = "postgres"
              url = "postgres://localhost:5432"
            }

            model Blog {
                id Int @id
            }
        "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_equals(&result)?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn updating_a_field_for_a_non_existent_column(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Blog {
            id Int @id
            title String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let initial_result = api.describe_database().await?;

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
        .await?;
    assert!(result.table_bang("Blog").column("title").is_none());

    let dm2 = r#"
        model Blog {
            id Int @id
            title Int @unique
        }
    "#;

    api.schema_push(dm2).force(true).send().await?;

    api.assert_schema().await?.assert_table("Blog", |table| {
        table
            .assert_column("title", |column| column.assert_type_is_int())?
            .assert_index_on_columns(&["title"], |idx| idx.assert_is_unique())
    })?;

    Ok(())
}

#[test_each_connector]
async fn renaming_a_field_where_the_column_was_already_renamed_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Blog {
            id Int @id @default(autoincrement())
            title String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let initial_result = api.assert_schema().await?.into_schema();
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
        .await?;

    assert!(result.table_bang("Blog").column("new_title").is_some());

    let dm2 = r#"
        model Blog {
            id Int @id @default(autoincrement())
            title Float @map(name: "new_title")
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    let final_result = api.assert_schema().await?.into_schema();
    let final_column = final_result.table_bang("Blog").column_bang("new_title");

    assert_eq!(final_column.tpe.family, ColumnTypeFamily::Float);
    assert!(final_result.table_bang("Blog").column("title").is_none());

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn existing_enums_are_picked_up(api: &TestApi) -> TestResult {
    let sql = r#"
        CREATE TYPE "Genre" AS ENUM ('SKA', 'PUNK');

        CREATE TABLE "prisma-tests"."Band" (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            genre "Genre" NOT NULL
        );
    "#;

    api.database().raw_cmd(sql).await?;

    let dm = r#"
        enum Genre {
            SKA
            PUNK
        }

        model Band {
            id Int @id @default(autoincrement())
            name String
            genre Genre
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}
