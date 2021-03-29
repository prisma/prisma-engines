use query_engine_tests::*;

#[test_suite(schema(schemas::user), capabilities(ScalarLists))]
mod scalar_list {}
