use migration_engine_tests::sync_test_api::*;

#[test_connector(tags(Mssql))]
fn shared_default_constraints_are_ignored_issue_5423(api: TestApi) {
    let schema = api.schema_name();

    api.raw_cmd(&format!("CREATE DEFAULT [{}].catcat AS 'musti'", schema));

    api.raw_cmd(&format!(
        r#"
                CREATE TABLE [{0}].cats (
                    id INT IDENTITY PRIMARY KEY,
                    name NVARCHAR(255) NOT NULL
                )
            "#,
        schema
    ));

    api.raw_cmd(&format!("sp_bindefault '{0}.catcat', '{0}.cats.name'", schema));

    let dm = api.datamodel_with_provider(
        r#"
        model cats {
            id Int @id @default(autoincrement())
            name String @db.NVarChar(255)
        }
    "#,
    );

    api.schema_push(dm)
        .migration_id(Some("first"))
        .send()
        .assert_green_bang()
        .assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn mssql_apply_migrations_error_output(api: TestApi) {
    let dm = "";
    let migrations_directory = api.create_migrations_directory();

    let migration = format!(
        r#"
        BEGIN TRY

        BEGIN TRAN;
        CREATE TABLE [{schema_name}].[Cat] ( id INT IDENTITY PRIMARY KEY );
        DROP TABLE [{schema_name}].[Emu];
        CREATE TABLE [{schema_name}].[Emu] (
            size INTEGER
        );
        COMMIT TRAN;

        END TRY

        BEGIN CATCH

        IF @@TRANCOUNT > 0
        BEGIN 
            ROLLBACK TRAN;
        END;
        THROW

        END CATCH
    "#,
        schema_name = api.schema_name()
    );

    let migration_name = api
        .create_migration("01init", dm, &migrations_directory)
        .draft(true)
        .send_sync()
        .modify_migration(|contents| {
            contents.clear();
            contents.push_str(&migration);
        })
        .into_output()
        .generated_migration_name
        .unwrap();

    let err = api
        .apply_migrations(&migrations_directory)
        .send_unwrap_err()
        .to_string()
        .replace(&migration_name, "<migration-name>");

    let expectation = expect![[r#"
        A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

        Migration name: <migration-name>

        Database error code: 3701

        Database error:
        Cannot drop the table 'mssql_apply_migrations_error_output.Emu', because it does not exist or you do not have permission.

    "#]];

    let first_segment = err
        .split_terminator("DbError {")
        .next()
        .unwrap()
        .split_terminator("   0: ")
        .next()
        .unwrap();

    expectation.assert_eq(first_segment)
}
