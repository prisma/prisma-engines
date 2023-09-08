use indoc::{formatdoc, indoc};
use sql_migration_tests::test_api::*;

#[test_connector(tags(Mssql))]
fn clustered_unique(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @unique(clustered: true)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["id"], |index| index.assert_clustered())
    });

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn non_clustered_unique(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @unique(clustered: false)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["id"], |index| index.assert_non_clustered())
    });

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn default_clustered_unique(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @unique
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["id"], |index| index.assert_non_clustered())
    });

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn non_clustered_to_clustered_unique(api: TestApi) {
    let schema = api.schema_name();

    let query = formatdoc!(
        r#"
        CREATE TABLE [{schema}].[A] (
            id INT NOT NULL,
            CONSTRAINT A_id_key UNIQUE NONCLUSTERED (id)
        );
    "#
    );

    api.raw_cmd(&query);

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["id"], |index| index.assert_non_clustered())
    });

    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @unique(clustered: true)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["id"], |index| index.assert_clustered())
    });
}

#[test_connector(tags(Mssql))]
fn clustered_to_non_clustered_unique(api: TestApi) {
    let schema = api.schema_name();

    let query = formatdoc!(
        r#"
        CREATE TABLE [{schema}].[A] (
            id INT NOT NULL,
            CONSTRAINT A_id_key UNIQUE CLUSTERED (id)
        );
    "#
    );

    api.raw_cmd(&query);

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["id"], |index| index.assert_clustered())
    });

    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @unique(clustered: false)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).force(true).send().assert_has_executed_steps();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["id"], |index| index.assert_non_clustered())
    });
}

#[test_connector(tags(Mssql))]
fn brownfield_unique_index_can_be_dropped(api: TestApi) {
    let schema = api.schema_name();

    let sql = formatdoc!(
        r#"
        CREATE TABLE [{schema}].[A] (
            id INT NOT NULL PRIMARY KEY,
            [left] INT,
            [right] INT
        );

        CREATE UNIQUE INDEX [A_left_right_idx] ON [{schema}].[A]([left], [right]);
    "#
    );

    api.raw_cmd(&sql);

    api.assert_schema().assert_table("A", |cat| {
        cat.assert_indexes_count(1)
            .assert_index_on_columns(&["left", "right"], |idx| idx.assert_is_unique())
    });

    let dm1 = indoc! {r#"
        model A {
          id    Int @id
          left  Int
          right Int
        }
    "#};

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |cat| cat.assert_indexes_count(0));
}

#[test_connector(tags(Mssql))]
fn creating_and_dropping_unique_constraint_works(api: TestApi) {
    let dm1 = indoc! {r#"
        model Logbook {
          id         Int      @id
          categoryId String?  @db.UniqueIdentifier
          date       DateTime @db.Date()
        }
    "#};

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema()
        .assert_table("Logbook", |cat| cat.assert_indexes_count(0));

    let dm1 = indoc! {r#"
        model Logbook {
          id         Int      @id
          categoryId String?  @db.UniqueIdentifier
          date       DateTime @db.Date()

          @@unique([categoryId, date])
        }
    "#};

    api.schema_push_w_datasource(dm1).force(true).send();

    api.assert_schema().assert_table("Logbook", |cat| {
        cat.assert_indexes_count(1)
            .assert_index_on_columns(&["categoryId", "date"], |idx| idx.assert_is_unique())
    });

    let dm2 = indoc! {r#"
        model Logbook {
          id         Int      @id
          categoryId String?  @db.UniqueIdentifier
          date       DateTime @db.Date()
        }
    "#};

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema()
        .assert_table("Logbook", |cat| cat.assert_indexes_count(0));
}

#[test_connector(tags(Mssql))]
fn dropping_unique_constraint_works(api: TestApi) {
    let dm1 = indoc! {r#"
        model Logbook {
          id         Int      @id
          categoryId String?  @db.UniqueIdentifier
          date       DateTime @db.Date()

          @@unique([categoryId, date])
        }
    "#};

    api.schema_push_w_datasource(dm1).force(true).send();

    api.assert_schema().assert_table("Logbook", |cat| {
        cat.assert_indexes_count(1)
            .assert_index_on_columns(&["categoryId", "date"], |idx| idx.assert_is_unique())
    });

    let dm2 = indoc! {r#"
        model Logbook {
          id         Int      @id
          categoryId String?  @db.UniqueIdentifier
          date       DateTime @db.Date()
        }
    "#};

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema()
        .assert_table("Logbook", |cat| cat.assert_indexes_count(0));
}
