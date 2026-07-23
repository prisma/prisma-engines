mod cockroachdb;
mod mssql;
mod vitess;

use indoc::formatdoc;
use sql_migration_tests::test_api::*;

#[test_connector(tags(Mysql8))]
fn length_prefixed_primary_key(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id String @id(length: 30) @db.Text
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_pk(|pk| pk.assert_column("id", |attr| attr.assert_length_prefix(30)))
    });
}

#[test_connector(tags(Mysql8))]
fn changing_of_length_prefix(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id String @id(length: 30) @db.VarChar(255)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_pk(|pk| pk.assert_column("id", |attr| attr.assert_length_prefix(30)))
    });

    let dm = formatdoc! {r#"
        {}

        model A {{
          id String @id(length: 20) @db.VarChar(255)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_pk(|pk| pk.assert_column("id", |attr| attr.assert_length_prefix(20)))
    });
}

#[test_connector(tags(Mysql8))]
fn removing_length_prefix(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id String @id(length: 30) @db.VarChar(255)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_pk(|pk| pk.assert_column("id", |attr| attr.assert_length_prefix(30)))
    });

    let dm = formatdoc! {r#"
        {}

        model A {{
          id String @id @db.VarChar(255)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_pk(|pk| pk.assert_column("id", |attr| attr.assert_no_length_prefix()))
    });
}

#[test_connector(tags(Mysql8))]
fn length_prefixed_compound_primary_key(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          a String @db.Text
          b String @db.Text

          @@id([a(length: 30), b(length: 20)])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_pk(|pk| {
            pk.assert_column("a", |attr| attr.assert_length_prefix(30))
                .assert_column("b", |attr| attr.assert_length_prefix(20))
        })
    });
}

// TODO: ignore because not possible on cockroachdb. We would need a multi-step process there.
#[test_connector(exclude(Vitess, CockroachDb))]
fn changing_the_type_of_an_id_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            b_id Int
            b  B   @relation(fields: [b_id], references: [id])
        }

        model B {
            id Int @id
            a  A[]
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_is_int())
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["id"]))
    });

    let dm2 = r#"
        model A {
            id Int @id
            b_id String
            b  B   @relation(fields: [b_id], references: [id])
        }

        model B {
            id String @id @default(cuid())
            a  A[]

        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_is_string())
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["id"]))
    });
}

#[test_connector(exclude(Sqlite, CockroachDb))]
fn models_with_an_autoincrement_field_as_part_of_a_multi_field_id_can_be_created(api: TestApi) {
    let dm = r#"
        model List {
            id        Int  @id @default(autoincrement())
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
            .assert_column("id", |col| col.assert_auto_increments())
    });
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_connector(exclude(Sqlite, CockroachDb))]
fn making_an_existing_id_field_autoincrement_works(api: TestApi) {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        Int        @id
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
            id        Int         @id @default(autoincrement())
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

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
// We test this separately on cockroachdb.
#[test_connector(exclude(Sqlite, CockroachDb))]
fn removing_autoincrement_from_an_existing_field_works(api: TestApi) {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        Int         @id @default(autoincrement())
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
    });

    // Data to see we don't lose anything in the translation.
    for content in &["A", "B", "C"] {
        let insert = Insert::single_into(api.render_table_name("Post")).value("content", *content);
        api.query(insert.into());
    }

    assert_eq!(
        3,
        api.query(Select::from_table(api.render_table_name("Post")).into())
            .len()
    );

    let dm2 = r#"
        model Post {
            id        Int         @id
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_no_autoincrement())
    });

    // Check that the migration is idempotent.
    api.schema_push_w_datasource(dm2)
        .migration_id(Some("idempotency-check"))
        .send()
        .assert_green()
        .assert_no_steps();

    assert_eq!(
        3,
        api.query(Select::from_table(api.render_table_name("Post")).into())
            .len()
    );
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
// Cockroachdb is tested separately.
#[test_connector(exclude(Sqlite, CockroachDb))]
fn flipping_autoincrement_on_and_off_works(api: TestApi) {
    let dm_without = r#"
        model Post {
            id        Int        @id
            title     String     @default("")
        }
    "#;

    let dm_with = r#"
        model Post {
            id        Int        @id @default(autoincrement())
            updatedAt DateTime
        }
    "#;

    for dm in [dm_with, dm_without].iter().cycle().take(5) {
        api.schema_push_w_datasource(*dm).send().assert_green();
    }
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
// Ignoring cockroachdb, because literal defaults on PKs on cockroachdb do not work.
#[test_connector(exclude(Sqlite, CockroachDb))]
fn making_an_autoincrement_default_an_expression_then_autoincrement_again_works(api: TestApi) {
    let dm1 = r#"
        model Post {
            id        Int        @id @default(autoincrement())
            title     String     @default("")
        }
    "#;

    api.schema_push_w_datasource(dm1)
        .migration_id(Some("apply_dm1"))
        .send()
        .assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
    });

    let dm2 = r#"
        model Post {
            id        Int       @id @default(3)
            title     String    @default("")
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .migration_id(Some("apply_dm2"))
        .send()
        .assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model
            .assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_no_autoincrement())
            .assert_column("id", |column| column.assert_int_default(3))
    });

    // Now re-apply the sequence.
    api.schema_push_w_datasource(dm1)
        .migration_id(Some("apply_dm1_again"))
        .send()
        .assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
    });
}

#[test_connector(exclude(CockroachDb))]
fn migrating_a_unique_constraint_to_a_primary_key_works(api: TestApi) {
    let dm = r#"
        model model1 {
            id              String        @id @default(cuid())
            a               String
            b               String
            c               String

            @@unique([a, b, c])

        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("model1", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["id"]))
            .assert_index_on_columns(&["a", "b", "c"], |idx| idx.assert_is_unique())
    });

    api.insert("model1")
        .value("id", "the-id")
        .value("a", "the-a")
        .value("b", "the-b")
        .value("c", "the-c")
        .result_raw();

    let dm2 = r#"
        model model1 {
            a               String
            b               String
            c               String

            @@id([a, b, c])

        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_executable()
        .assert_warnings(&["The primary key for the `model1` table will be changed. If it partially fails, the table could be left without primary key constraint.".into(), "You are about to drop the column `id` on the `model1` table, which still contains 1 non-null values.".into()]);

    api.assert_schema().assert_table("model1", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["a", "b", "c"]))
    });
}
