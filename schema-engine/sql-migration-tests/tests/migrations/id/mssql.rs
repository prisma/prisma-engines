use indoc::{formatdoc, indoc};
use sql_migration_tests::test_api::*;
use sql_schema_describer::SQLSortOrder;

#[test_connector(tags(Mssql))]
fn non_clustered_id(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id(clustered: false)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| table.assert_pk(|pk| pk.assert_non_clustered()));

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn non_clustered_to_clustered_change(api: TestApi) {
    let schema = api.schema_name();

    let query =
        format!("CREATE TABLE [{schema}].[A] (id INT NOT NULL, CONSTRAINT A_pkey PRIMARY KEY NONCLUSTERED (id))");

    api.raw_cmd(&query);

    api.assert_schema()
        .assert_table("A", |table| table.assert_pk(|pk| pk.assert_non_clustered()));

    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm)
        .force(true)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema()
        .assert_table("A", |table| table.assert_pk(|pk| pk.assert_clustered()));

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn clustered_to_non_clustered_change(api: TestApi) {
    let schema = api.schema_name();

    let query = format!("CREATE TABLE [{schema}].[A] (id INT NOT NULL, CONSTRAINT A_pkey PRIMARY KEY CLUSTERED (id))");

    api.raw_cmd(&query);

    api.assert_schema()
        .assert_table("A", |table| table.assert_pk(|pk| pk.assert_clustered()));

    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id(clustered: false)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm)
        .force(true)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema()
        .assert_table("A", |table| table.assert_pk(|pk| pk.assert_non_clustered()));

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn clustered_id(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id(clustered: true)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| table.assert_pk(|pk| pk.assert_clustered()));

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn default_id_clustering(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| table.assert_pk(|pk| pk.assert_clustered()));

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn non_clustered_compound_id(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          a Int
          b Int

          @@id([a, b], clustered: false)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| table.assert_pk(|pk| pk.assert_non_clustered()));

    api.schema_push(&dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mssql))]
fn descending_primary_key(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id(sort: Desc)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_pk(|pk| pk.assert_column("id", |attr| attr.assert_sort_order(SQLSortOrder::Desc)))
    });
}

#[test_connector(tags(Mssql))]
fn altering_descending_primary_key(api: TestApi) {
    let dm = indoc! {r#"
        datasource slqserverdb {
            provider = "sqlserver"
        }

        model A {
          id Int @id(sort: Desc)
        }
    "#};

    api.schema_push(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_pk(|pk| pk.assert_column("id", |attr| attr.assert_sort_order(SQLSortOrder::Desc)))
    });

    let dm = indoc! {r#"
        datasource slqserverdb {
            provider = "sqlserver"
        }

        model A {
          id Int @id
        }
    "#};

    api.schema_push(dm)
        .force(true)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("A", |table| {
        table.assert_pk(|pk| pk.assert_column("id", |attr| attr.assert_sort_order(SQLSortOrder::Asc)))
    });
}

#[test_connector(tags(Mssql))]
fn making_an_existing_id_field_autoincrement_works_with_indices(api: TestApi) {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        Int        @id
            content   String?

            @@index([content], name: "fooBarIndex")
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model
            .assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_no_autoincrement())
            .assert_indexes_count(1)
    });

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

    let dm2 = r#"
        model Post {
            id        Int         @id @default(autoincrement())
            content   String?

            @@index([content], name: "fooBarIndex")
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model
            .assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
            .assert_indexes_count(1)
    });

    // Check that the migration is idempotent.
    api.schema_push_w_datasource(dm2)
        .send()
        .assert_green()
        .assert_no_steps();

    assert_eq!(
        3,
        api.query(Select::from_table(api.render_table_name("Post")).into())
            .len()
    );
}

#[test_connector(tags(Mssql))]
fn making_an_existing_id_field_autoincrement_works_with_foreign_keys(api: TestApi) {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        Int         @id
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
            author_id Int
            author    Author      @relation(fields: [author_id], references: [id])
            trackings Tracking[]
        }

        model Tracking {
            id        Int         @id @default(autoincrement())
            post_id   Int
            post      Post        @relation(fields: [post_id], references: [id])
        }

        model Author {
            id        Int         @id @default(autoincrement())
            posts     Post[]
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_no_autoincrement())
    });

    // Data to see we don't lose anything in the translation.
    for (i, content) in ["A", "B", "C"].iter().enumerate() {
        let insert = Insert::single_into(api.render_table_name("Author"));

        let author_id = api
            .query(Insert::from(insert).returning(["id"]).into())
            .into_single()
            .unwrap()
            .into_single()
            .unwrap()
            .as_integer()
            .unwrap();

        let insert = Insert::single_into(api.render_table_name("Post"))
            .value("content", *content)
            .value("id", i)
            .value("author_id", author_id);

        api.query(insert.into());

        let insert = Insert::single_into(api.render_table_name("Tracking")).value("post_id", i);

        api.query(insert.into());
    }

    assert_eq!(
        3,
        api.query(Select::from_table(api.render_table_name("Post")).into())
            .len()
    );

    let dm2 = r#"
        model Post {
            id        Int         @id @default(autoincrement())
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
            author_id Int
            author    Author      @relation(fields: [author_id], references: [id])
            trackings Tracking[]
        }

        model Tracking {
            id        Int         @id @default(autoincrement())
            post_id   Int
            post      Post        @relation(fields: [post_id], references: [id])
        }

        model Author {
            id        Int         @id @default(autoincrement())
            posts     Post[]
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_autoincrement())
    });

    // // TODO: Check that the migration is idempotent.
    // api.schema_push(dm2).send_sync().assert_green_bang().assert_no_steps();

    assert_eq!(
        3,
        api.query(Select::from_table(api.render_table_name("Post")).into())
            .len()
    );
}
