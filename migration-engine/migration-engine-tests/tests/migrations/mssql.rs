use migration_engine_tests::sql::*;
use quaint::connector::Queryable;

#[test_each_connector(tags("mssql"), log = "debug")]
async fn shared_default_constraints_are_ignored_issue_5423(api: &TestApi) -> TestResult {
    api.database()
        .raw_cmd(&format!("CREATE DEFAULT [{}].catcat AS 'musti'", api.schema_name()))
        .await?;

    api.database()
        .raw_cmd(&format!(
            r#"
                CREATE TABLE [{0}].cats (
                    id INT IDENTITY PRIMARY KEY,
                    name NVARCHAR(255) NOT NULL
                )
            "#,
            api.schema_name()
        ))
        .await?;

    api.database()
        .raw_cmd(&format!(
            "sp_bindefault '{0}.catcat', '{0}.cats.name'",
            api.schema_name()
        ))
        .await?;

    let dm = api.native_types_datamodel(
        r#"
        model cats {
            id Int @id @default(autoincrement())
            name String @test_db.NVarChar(255)
        }
    "#,
    );

    api.schema_push(dm)
        .migration_id(Some("first"))
        .send()
        .await?
        .assert_green()?
        .assert_no_steps()?;

    Ok(())
}
