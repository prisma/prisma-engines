use migration_engine_tests::sync_test_api::*;

#[test_connector(tags(Mssql))]
fn shared_default_constraints_are_ignored_issue_5423(api: TestApi) {
    let schema = api.connection_info().schema_name();

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

    let dm = format!(
        r#"
        {}

        model cats {{
            id Int @id @default(autoincrement())
            name String @db.NVarChar(255)
        }}
    "#,
        api.datasource_block(),
    );

    api.schema_push(dm)
        .migration_id(Some("first"))
        .send_sync()
        .assert_green_bang()
        .assert_no_steps();
}
