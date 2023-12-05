use sql_migration_tests::test_api::*;
use sql_schema_describer::postgres::SqlIndexAlgorithm;

#[test_connector(tags(CockroachDb))]
fn gin_preview_disabled(api: TestApi) {
    let schema_name = api.schema_name();
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id SERIAL PRIMARY KEY, data int4[] not null)",);
    let create_idx = format!("CREATE INDEX \"A_data_idx\" ON \"{schema_name}\".\"A\" USING GIN (data);",);

    api.raw_cmd(&create_table);
    api.raw_cmd(&create_idx);

    let dm = r#"
        model A {
          id   Int   @id
          data Int[]

          @@index([data])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(CockroachDb), preview_features("cockroachDb"))]
fn gin_array_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id
          data Int[]

          @@index([data], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SqlIndexAlgorithm::Gin))
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(CockroachDb), preview_features("cockroachDb"))]
fn gin_jsonb_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int  @id
          data Json

          @@index([data], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SqlIndexAlgorithm::Gin))
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(CockroachDb), preview_features("cockroachDb"))]
fn gin_raw_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int       @id
          data Geometry?

          @@index([data], type: Gin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SqlIndexAlgorithm::Gin))
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}
