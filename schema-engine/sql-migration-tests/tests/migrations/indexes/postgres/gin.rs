use sql_migration_tests::test_api::*;
use sql_schema_describer::postgres::{SQLOperatorClassKind, SqlIndexAlgorithm};

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gin_change_from_btree(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data Int[]

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
          data Int[]

          @@index([data(ops: ArrayOps)], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SqlIndexAlgorithm::Gin))
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gin_array_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Int[]

          @@index([data(ops: ArrayOps)], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::ArrayOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gin_array_default_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data String[] @db.Uuid

          @@index([data], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::ArrayOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gin_array_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Int[]

          @@index([data], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::ArrayOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gin_jsonb_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Json

          @@index([data(ops: JsonbOps)], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::JsonbOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gin_jsonb_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Json

          @@index([data], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::JsonbOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gin_jsonb_path_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Json

          @@index([data(ops: JsonbPathOps)], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::JsonbPathOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn from_jsonb_ops_to_jsonb_path_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Json

          @@index([data(ops: JsonbOps)], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let dm = r#"
        model A {
          id   Int  @id @default(autoincrement())
          data Json

          @@index([data(ops: JsonbPathOps)], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::JsonbPathOps))
            })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn compound_index_with_different_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data Json
          sata String[]

          @@index([data(ops: JsonbOps), sata(ops: ArrayOps)], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data", "sata"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::JsonbOps))
                    .assert_column("sata", |attrs| attrs.assert_ops(SQLOperatorClassKind::ArrayOps))
            })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gin_raw_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int                      @id @default(autoincrement())
          data Unsupported("tsvector")?

          @@index([data(raw: "tsvector_ops")], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::raw("tsvector_ops"))
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn gin_raw_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int                      @id @default(autoincrement())
          data Unsupported("tsvector")?

          @@index([data], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Gin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::raw("tsvector_ops"))
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}
