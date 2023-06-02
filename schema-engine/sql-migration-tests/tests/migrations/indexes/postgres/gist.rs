use sql_migration_tests::test_api::*;
use sql_schema_describer::postgres::{SQLOperatorClassKind, SqlIndexAlgorithm};

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gist_change_from_btree(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Inet

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
          id   Int    @id @default(autoincrement())
          data String @db.Inet

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
fn gist_inet_ops(api: TestApi) {
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
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::InetOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gist_raw_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int                      @id @default(autoincrement())
          data Unsupported("tsvector")?

          @@index([data(raw: "tsvector_ops")], type: Gist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gist)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::raw("tsvector_ops"))
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gist_unsupported_no_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int                      @id @default(autoincrement())
          data Unsupported("tsvector")?

          @@index([data], type: Gist)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gist)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::raw("tsvector_ops"))
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}
