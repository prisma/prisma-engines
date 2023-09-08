use indoc::formatdoc;
use sql_migration_tests::test_api::*;

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
