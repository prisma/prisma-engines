use barrel::types;
use migration_engine_tests::sql::*;
use quaint::prelude::Queryable;
use sql_schema_describer::*;

#[test_connector]
async fn adding_a_model_for_an_existing_table_must_work(api: &TestApi) -> TestResult {
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
            id Int @id @default(autoincrement())
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_equals(&initial_result.into_schema())?;

    Ok(())
}

#[test_connector]
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
    api.assert_schema().await?.assert_has_table("Post")?;

    let result = api
        .barrel()
        .execute(|migration| {
            migration.drop_table("Post");
        })
        .await;

    let result = result.assert_tables_count(1)?.assert_has_table("Blog")?;

    let dm2 = r#"
        model Blog {
            id Int @id
        }
    "#;

    api.schema_push(dm2).send().await?;

    api.assert_schema().await?.assert_equals(&result.into_schema())?;

    Ok(())
}

#[test_connector]
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
        .await;

    let dm = r#"
        model Blog {
            id      Int @id @default(autoincrement())
            title   String
        }
    "#;

    api.schema_push(dm).force(true).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_equals(&initial_result.into_schema())?;

    Ok(())
}

#[test_connector]
async fn creating_a_field_for_an_existing_column_and_changing_its_type_must_work(api: &TestApi) -> TestResult {
    let initial_result = api
        .barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("title", types::integer().nullable(true));
            });
        })
        .await;

    initial_result.assert_table("Blog", |t| {
        t.assert_column("title", |c| c.assert_type_is_int()?.assert_is_nullable())
    })?;

    let dm = r#"
            model Blog {
                id Int @id
                title String @unique
            }
        "#;

    api.schema_push(dm).force(true).send().await?;
    api.assert_schema().await?.assert_table("Blog", |t| {
        t.assert_column("title", |c| c.assert_type_is_string()?.assert_is_required())?
            .assert_index_on_columns(&["title"], |idx| idx.assert_is_unique())
    })?;

    Ok(())
}

#[test_connector]
async fn creating_a_field_for_an_existing_column_and_simultaneously_making_it_optional(api: &TestApi) -> TestResult {
    let initial_result = api
        .barrel()
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("title", types::text());
            });
        })
        .await;

    initial_result.assert_table("Blog", |t| t.assert_column("title", |c| c.assert_is_required()))?;

    let dm = r#"
        model Blog {
            id Int @id
            title String?
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Blog", |t| t.assert_column("title", |c| c.assert_is_nullable()))?;

    Ok(())
}

#[test_connector(capabilities(ScalarLists))]
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
    api.assert_schema().await?.assert_has_table("Blog")?;

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

    api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps();
    api.assert_schema().await?.assert_equals(&result.into_schema())?;

    Ok(())
}

#[test_connector]
async fn delete_a_field_for_a_non_existent_column_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
            model Blog {
                id      Int @id
                title   String
            }
        "#;

    api.schema_push(dm1).send().await?;
    api.assert_schema()
        .await?
        .assert_table("Blog", |t| t.assert_columns_count(2)?.assert_has_column("title"))?;

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

    api.assert_schema()
        .await?
        .assert_table("Blog", |t| t.assert_columns_count(1)?.assert_has_column("id"))?;

    let dm2 = r#"
        model Blog {
            id Int @id @default(autoincrement())
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps();
    api.assert_schema().await?.assert_equals(&result.into_schema())?;

    Ok(())
}

#[test_connector(capabilities(ScalarLists))]
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

    api.schema_push(dm2).send().await?.assert_green()?;
    api.assert_schema().await?.assert_equals(&result.into_schema())?;

    Ok(())
}

#[test_connector]
async fn updating_a_field_for_a_non_existent_column(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Blog {
            id Int @id
            title String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("Blog", |t| t.assert_column("title", |c| c.assert_type_is_string()))?;

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

    result.assert_table("Blog", |t| t.assert_columns_count(1)?.assert_has_column("id"))?;

    let dm2 = r#"
        model Blog {
            id Int @id
            title Int @unique
        }
    "#;

    api.schema_push(dm2).force(true).send().await?;
    api.assert_schema().await?.assert_table("Blog", |t| {
        t.assert_column("title", |c| c.assert_type_is_int())?
            .assert_index_on_columns(&["title"], |idx| idx.assert_is_unique())
    })?;

    Ok(())
}

#[test_connector]
async fn renaming_a_field_where_the_column_was_already_renamed_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Blog {
            id Int @id @default(autoincrement())
            title String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("Blog", |t| t.assert_column("title", |c| c.assert_type_is_string()))?;

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

    result.assert_table("Blog", |t| t.assert_has_column("new_title"))?;

    let dm2 = r#"
        model Blog {
            id Int @id @default(autoincrement())
            title Float @map(name: "new_title")
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Blog", |t| {
        t.assert_column("new_title", |c| c.assert_type_family(ColumnTypeFamily::Float))?
            .assert_columns_count(2)?
            .assert_has_column("id")
    })?;

    Ok(())
}

#[test_connector(tags(Postgres))]
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

    api.schema_push(dm).send().await?.assert_green()?.assert_no_steps();

    Ok(())
}
