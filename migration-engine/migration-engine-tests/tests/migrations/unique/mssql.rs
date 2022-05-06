use indoc::formatdoc;
use migration_engine_tests::test_api::*;

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
