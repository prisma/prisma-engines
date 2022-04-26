use migration_engine_tests::test_api::*;
use sql_schema_describer::{SQLIndexAlgorithm, SQLOperatorClassKind};

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gist_preview_disabled(api: TestApi) {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data inet)",);
    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIST (data inet_ops);",);

    api.raw_cmd(&create_table);
    api.raw_cmd(&create_idx);

    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Inet

          @@index([data])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("extendedIndexes"))]
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
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SQLIndexAlgorithm::BTree))
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
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SQLIndexAlgorithm::Gist))
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("extendedIndexes"))]
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
                idx.assert_algorithm(SQLIndexAlgorithm::Gist)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::InetOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("extendedIndexes"))]
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
                idx.assert_algorithm(SQLIndexAlgorithm::Gist)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::raw("tsvector_ops"))
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("extendedIndexes"))]
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
                idx.assert_algorithm(SQLIndexAlgorithm::Gist)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::raw("tsvector_ops"))
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}
