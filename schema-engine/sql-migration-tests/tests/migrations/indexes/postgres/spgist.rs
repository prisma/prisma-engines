use sql_migration_tests::test_api::*;
use sql_schema_describer::postgres::{SQLOperatorClassKind, SqlIndexAlgorithm};

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn spgist_change_from_btree(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SqlIndexAlgorithm::BTree))
    });

    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data(ops: InetOps)], type: Gist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SqlIndexAlgorithm::Gist))
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn spgist_inet_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data(ops: InetOps)], type: SpGist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::SpGist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::InetOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn spgist_inet_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data], type: SpGist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::SpGist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::InetOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn spgist_text_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String

          @@index([data(ops: TextOps)], type: SpGist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::SpGist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn spgist_text_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String

          @@index([data], type: SpGist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::SpGist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn spgist_text_ops_varchar(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.VarChar(255)

          @@index([data(ops: TextOps)], type: SpGist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::SpGist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn spgist_text_ops_varchar_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.VarChar(255)

          @@index([data], type: SpGist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::SpGist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn spgist_raw_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int                 @id @default(autoincrement())
          data Unsupported("box")?

          @@index([data(ops: raw("box_ops"))], type: SpGist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::SpGist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::raw("box_ops")))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn spgist_raw_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int                 @id @default(autoincrement())
          data Unsupported("box")?

          @@index([data], type: SpGist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::SpGist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::raw("box_ops")))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}
