mod brin;
mod gin;
mod gist;
mod spgist;

use migration_engine_tests::test_api::*;
use sql_schema_describer::SQLIndexAlgorithm;

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("extendedIndexes"))]
fn hash_index(api: TestApi) {
    let dm = r#"
        model A {
          id Int @id
          a  Int

          @@index([a], type: Hash)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a"], |index| {
            index.assert_is_not_unique().assert_algorithm(SQLIndexAlgorithm::Hash)
        })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}
