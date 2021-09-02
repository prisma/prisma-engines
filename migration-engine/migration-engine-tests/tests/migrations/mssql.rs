use migration_connector::DiffTarget;
use migration_engine_tests::test_api::*;

#[test_connector(tags(Mssql))]
fn reset_clears_udts(api: TestApi) {
    let schema = api.schema_name();

    api.raw_cmd(&format!("CREATE TYPE {}.[testType] AS TABLE (FooBar INT)", schema));

    let schemas = api.query_raw(
        &format!(
            "SELECT * FROM sys.types WHERE SCHEMA_NAME(schema_id) = '{}' and NAME = 'testType'",
            schema
        ),
        &[],
    );
    assert_eq!(1, schemas.len());

    api.reset().send_sync();

    let schemas = api.query_raw(
        &format!(
            "SELECT * FROM sys.types WHERE SCHEMA_NAME(schema_id) = '{}' and NAME = 'testType'",
            schema
        ),
        &[],
    );
    assert_eq!(0, schemas.len());
}

#[test_connector(tags(Mssql))]
fn shared_default_constraints_are_ignored_issue_5423(api: TestApi) {
    let schema = api.schema_name();

    api.raw_cmd(&format!("CREATE DEFAULT [{}].catcat AS 'musti'", schema));

    api.raw_cmd(&format!(
        r#"
            CREATE TABLE [{0}].cats (
                id INT IDENTITY,
                name NVARCHAR(255) NOT NULL,
                CONSTRAINT [cats_pkey] PRIMARY KEY CLUSTERED ([id] ASC)
            )
        "#,
        schema
    ));

    api.raw_cmd(&format!("sp_bindefault '{0}.catcat', '{0}.cats.name'", schema));

    let dm = r#"
        model cats {
            id Int @id @default(autoincrement())
            name String @db.NVarChar(255)
        }
    "#;

    api.schema_push_w_datasource(dm)
        .migration_id(Some("first"))
        .send()
        .assert_green()
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
        Cannot drop the table 'mssql_apply_migrations_error_output.Emu', because it does not exist or you do not have permission."#]];

    let first_segment = err
        .split_terminator("   0: ")
        .next()
        .unwrap()
        .trim_end_matches(|c| c == '\n' || c == ' ');

    expectation.assert_eq(first_segment)
}

#[test_connector(tags(Mssql))]
fn foreign_key_renaming_to_default_works(api: TestApi) {
    let setup = format!(
        r#"
        CREATE TABLE [{schema}].[food] (
            id INTEGER IDENTITY,
            CONSTRAINT [food_pkey] PRIMARY KEY (id)
        );

        CREATE TABLE [{schema}].[Dog] (
            id INTEGER IDENTITY,
            favourite_food_id INTEGER NOT NULL,
            CONSTRAINT [Dog_pkey] PRIMARY KEY (id),
            CONSTRAINT [favouriteFood] FOREIGN KEY ([favourite_food_id])
                    REFERENCES [{schema}].[food]([id])
                    ON UPDATE NO ACTION
                    ON DELETE NO ACTION
        );
        "#,
        schema = api.schema_name(),
    );

    api.raw_cmd(&setup);

    let target_schema = r#"
        datasource db {
            provider = "sqlserver"
            url = env("TEST_DATABASE_URL")
        }

        model Dog {
            id                  Int @id @default(autoincrement())
            favourite_food_id   Int
            favouriteFood       food @relation(fields: [favourite_food_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model food {
            id      Int @id @default(autoincrement())
            dogs    Dog[]
        }
    "#;

    let parsed = datamodel::parse_schema(target_schema).unwrap();
    let migration = api.diff(DiffTarget::Database, DiffTarget::Datamodel((&parsed.0, &parsed.1)));
    let expected = expect![[r#"
        BEGIN TRY

        BEGIN TRAN;

        -- RenameForeignKey
        EXEC sp_rename 'foreign_key_renaming_to_default_works.favouriteFood', 'Dog_favourite_food_id_fkey', 'OBJECT';

        COMMIT TRAN;

        END TRY
        BEGIN CATCH

        IF @@TRANCOUNT > 0
        BEGIN
            ROLLBACK TRAN;
        END;
        THROW

        END CATCH
    "#]];

    expected.assert_eq(&migration);

    // Check that the migration applies cleanly.
    api.raw_cmd(&migration);

    // Check that the migration is idempotent.
    api.schema_push(target_schema).send().assert_green().assert_no_steps();
}
