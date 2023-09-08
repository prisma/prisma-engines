use sql_migration_tests::test_api::*;
use sql_schema_describer::DefaultKind;

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_connector(tags(CockroachDb))]
fn flipping_autoincrement_on_and_off_works(api: TestApi) {
    let dm_without = r#"
        model Post {
            id        BigInt  @id
            title     String     @default("")
        }
    "#;

    let dm_with = r#"
        model Post {
            id        BigInt        @id @default(autoincrement())
            updatedAt DateTime
        }
    "#;

    for dm in [dm_with, dm_without].iter().cycle().take(5) {
        api.schema_push_w_datasource(*dm).send().assert_green();
    }
}

#[test_connector(tags(CockroachDb))]
fn models_with_an_autoincrement_field_as_part_of_a_multi_field_id_can_be_created(api: TestApi) {
    let dm = r#"
        model List {
            id        BigInt  @id @default(autoincrement())
            uList     String? @unique
            todoId    BigInt @default(1)
            todoName  String
            todo      Todo   @relation(fields: [todoId, todoName], references: [id, uTodo])
        }

        model Todo {
            id     BigInt @default(autoincrement())
            uTodo  String
            lists  List[]

            @@id([id, uTodo])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Todo", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["id", "uTodo"]))
            .assert_column("id", |col| col.assert_default_kind(Some(DefaultKind::UniqueRowid)))
    });
}

#[test_connector(tags(CockroachDb))]
fn making_an_existing_id_field_autoincrement_works(api: TestApi) {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        BigInt        @id
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_no_autoincrement())
    });

    // MySQL cannot add autoincrement property to a column that already has data.
    if !api.is_mysql() {
        // Data to see we don't lose anything in the translation.
        for (i, content) in ["A", "B", "C"].iter().enumerate() {
            let insert = Insert::single_into(api.render_table_name("Post"))
                .value("content", *content)
                .value("id", i);

            api.query(insert.into());
        }

        assert_eq!(
            3,
            api.query(Select::from_table(api.render_table_name("Post")).into())
                .len()
        );
    }

    let dm2 = r#"
        model Post {
            id        BigInt         @id @default(autoincrement())
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
    });

    // Check that the migration is idempotent.
    api.schema_push_w_datasource(dm2)
        .send()
        .assert_green()
        .assert_no_steps();

    // MySQL cannot add autoincrement property to a column that already has data.
    if !api.is_mysql() {
        assert_eq!(
            3,
            api.query(Select::from_table(api.render_table_name("Post")).into())
                .len()
        );
    }
}
