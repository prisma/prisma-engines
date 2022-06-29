use indoc::{formatdoc, indoc};
use migration_engine_tests::test_api::*;

#[test_connector(tags(Mssql))]
fn clustered_index(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id(clustered: false)
          og Int

          @@index([og], clustered: true)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["og"], |index| index.assert_clustered())
    });

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn non_clustered_index(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id
          og Int

          @@index([og], clustered: false)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["og"], |index| index.assert_non_clustered())
    });

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn default_clustered_index(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id
          og Int

          @@index([og])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["og"], |index| index.assert_non_clustered())
    });

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn clustered_index_no_preview(api: TestApi) {
    let schema = api.schema_name();

    let query = formatdoc!(
        r#"
        CREATE TABLE [{schema}].[A] (
            id INT NOT NULL,
            val INT NOT NULL,
            CONSTRAINT A_pkey PRIMARY KEY NONCLUSTERED (id)
        );

        CREATE CLUSTERED INDEX A_val_idx ON [{schema}].[A] (val)
    "#
    );

    api.raw_cmd(&query);

    let dm = formatdoc! {r#"
        {}

        model A {{
          id  Int @id @default(autoincrement())
          val Int

          @@index([val])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["val"], |index| index.assert_non_clustered())
    });

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn non_clustered_to_clustered_index(api: TestApi) {
    let schema = api.schema_name();

    let query = formatdoc!(
        r#"
        CREATE TABLE [{schema}].[A] (
            id INT NOT NULL,
            og INT NOT NULL,
            CONSTRAINT A_pkey PRIMARY KEY NONCLUSTERED (id)
        );

        CREATE NONCLUSTERED INDEX A_og_idx ON [{schema}].[A] (og)
    "#
    );

    api.raw_cmd(&query);

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["og"], |index| index.assert_non_clustered())
    });

    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id(clustered: false)
          og Int

          @@index([og], clustered: true)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["og"], |index| index.assert_clustered())
    });
}

#[test_connector(tags(Mssql))]
fn clustered_to_non_clustered_index(api: TestApi) {
    let schema = api.schema_name();

    let query = formatdoc!(
        r#"
        CREATE TABLE [{schema}].[A] (
            id INT NOT NULL,
            og INT NOT NULL,
            CONSTRAINT A_pkey PRIMARY KEY NONCLUSTERED (id)
        );

        CREATE CLUSTERED INDEX A_og_idx ON [{schema}].[A] (og)
    "#
    );

    api.raw_cmd(&query);

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["og"], |index| index.assert_clustered())
    });

    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id(clustered: false)
          og Int

          @@index([og], clustered: false)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["og"], |index| index.assert_non_clustered())
    });
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
