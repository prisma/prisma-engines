use psl::parser_database::SourceFile;
use schema_core::schema_connector::DiffTarget;
use sql_migration_tests::test_api::*;

mod multi_schema;

#[test_connector(tags(Mssql))]
fn reset_clears_udts(api: TestApi) {
    let schema = api.schema_name();

    api.raw_cmd(&format!("CREATE TYPE {schema}.[testType] AS TABLE (FooBar INT)"));

    let schemas = api.query_raw(
        &format!("SELECT * FROM sys.types WHERE SCHEMA_NAME(schema_id) = '{schema}' and NAME = 'testType'"),
        &[],
    );
    assert_eq!(1, schemas.len());

    api.reset().send_sync(None);

    let schemas = api.query_raw(
        &format!("SELECT * FROM sys.types WHERE SCHEMA_NAME(schema_id) = '{schema}' and NAME = 'testType'"),
        &[],
    );
    assert_eq!(0, schemas.len());
}

#[test_connector(tags(Mssql))]
fn shared_default_constraints_are_ignored_issue_5423(api: TestApi) {
    let schema = api.schema_name();

    api.raw_cmd(&format!("CREATE DEFAULT [{schema}].catcat AS 'musti'"));

    api.raw_cmd(&format!(
        r#"
            CREATE TABLE [{schema}].cats (
                id INT IDENTITY,
                name NVARCHAR(255) NOT NULL,
                CONSTRAINT [cats_pkey] PRIMARY KEY CLUSTERED ([id] ASC)
            )
        "#
    ));

    api.raw_cmd(&format!("sp_bindefault '{schema}.catcat', '{schema}.cats.name'"));

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
fn shared_default_constraints_with_multilines_are_ignored_issue_24275(api: TestApi) {
    let schema = api.schema_name();

    api.raw_cmd(&format!(
        r#"
        /* This is a comment */
        CREATE DEFAULT [{schema}].dogdog AS 'mugi'
        "#
    ));

    api.raw_cmd(&format!(
        r#"
            CREATE TABLE [{schema}].dogs (
                id INT IDENTITY,
                name NVARCHAR(255) NOT NULL,
                CONSTRAINT [dogs_pkey] PRIMARY KEY CLUSTERED ([id] ASC)
            )
        "#
    ));

    api.raw_cmd(&format!("sp_bindefault '{schema}.dogdog', '{schema}.dogs.name'"));

    let dm = r#"
        model dogs {
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
        Cannot drop the table 'dbo.Emu', because it does not exist or you do not have permission."#]];

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

    let migration = api.connector_diff(
        DiffTarget::Database,
        DiffTarget::Datamodel(vec![(
            "schema.prisma".to_string(),
            SourceFile::new_static(target_schema),
        )]),
        None,
    );
    let expected = expect![[r#"
        BEGIN TRY

        BEGIN TRAN;

        -- RenameForeignKey
        EXEC sp_rename 'dbo.favouriteFood', 'Dog_favourite_food_id_fkey', 'OBJECT';

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

// Root cause: redefining a table recreated all the foreign keys pointing to that table, but if we
// were creating them for the first time in the migration in the first place (here: new table), the
// foreign key is created twice and that conflicts.
#[test_connector(tags(Mssql))]
fn prisma_9537(api: TestApi) {
    let schema = r#"
        datasource db {
            provider = "sqlserver"
            url = env("DBURL")
        }

        model User {
          id   Int    @id
          name String
        }
    "#;

    api.schema_push(schema)
        .migration_id(Some("first migration"))
        .send()
        .assert_green();

    let schema = r#"
        datasource db {
            provider = "sqlserver"
            url = env("DBURL")
        }

        model User {
          id    Int     @id @default(autoincrement())
          email String  @unique
          name  String?
          posts Post[]
        }

        model Post {
          id        Int      @id @default(autoincrement())
          title     String
          content   String?
          published Boolean  @default(false)
          author    User?    @relation(fields: [authorId], references: [id])
          authorId  Int?
        }
    "#;

    api.schema_push(schema)
        .migration_id(Some("second migration"))
        .send()
        .assert_green();
}

#[test_connector(tags(Mssql))]
fn bigint_defaults_work(api: TestApi) {
    let schema = r#"
        datasource mypg {
            provider = "sqlserver"
            url = env("TEST_DATABASE_URL")
        }

        model foo {
          id  String @id
          bar BigInt @default(0)
        }
    "#;
    let sql = expect![[r#"
        BEGIN TRY

        BEGIN TRAN;

        -- CreateTable
        CREATE TABLE [dbo].[foo] (
            [id] NVARCHAR(1000) NOT NULL,
            [bar] BIGINT NOT NULL CONSTRAINT [foo_bar_df] DEFAULT 0,
            CONSTRAINT [foo_pkey] PRIMARY KEY CLUSTERED ([id])
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
    "#]];
    api.expect_sql_for_schema(schema, &sql);

    api.schema_push(schema).send().assert_green();
    api.schema_push(schema).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn float_columns(api: TestApi) {
    let schema = r#"
        datasource mypg {
            provider = "sqlserver"
            url = env("TEST_DATABASE_URL")
        }

        model foo {
          id  String @id
          bar Float @mypg.Float @default(0.90001)
          baz Float? @mypg.Float
          qux Float? @mypg.Real
        }
    "#;
    let sql = expect![[r#"
        BEGIN TRY

        BEGIN TRAN;

        -- CreateTable
        CREATE TABLE [dbo].[foo] (
            [id] NVARCHAR(1000) NOT NULL,
            [bar] FLOAT NOT NULL CONSTRAINT [foo_bar_df] DEFAULT 0.90001,
            [baz] FLOAT,
            [qux] REAL,
            CONSTRAINT [foo_pkey] PRIMARY KEY CLUSTERED ([id])
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
    "#]];
    api.expect_sql_for_schema(schema, &sql);

    api.schema_push(schema).send().assert_green();
    api.schema_push(schema).send().assert_green().assert_no_steps();
}
