use migration_core::migration_api;
use migration_engine_tests::multi_engine_test_api::*;
use test_macros::test_connector;
use url::Url;

#[test_connector(tags(Postgres))]
fn connecting_to_a_postgres_database_with_missing_schema_creates_it(api: TestApi) {
    // Check that the "unexpected" schema does not exist.
    {
        let schema_exists_result = api
            .query_raw(
                "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = 'unexpected')",
                &[],
            )
            .unwrap();

        let schema_exists = schema_exists_result
            .into_single()
            .unwrap()
            .at(0)
            .unwrap()
            .as_bool()
            .unwrap();

        assert!(!schema_exists)
    }

    // Connect to the database with the wrong schema
    {
        let mut url: Url = api.connection_string().parse().unwrap();

        let mut new_qs = String::with_capacity(url.query().map(|q| q.len()).unwrap_or(16));

        for (k, v) in url.query_pairs() {
            if k == "schema" {
                new_qs.push_str("schema=unexpected&");
            } else {
                new_qs.push_str(&k);
                new_qs.push('=');
                new_qs.push_str(&v);
                new_qs.push('&');
            }
        }

        url.set_query(Some(new_qs.trim_end_matches('&')));

        let datamodel = format!(
            r#"
                datasource db {{
                    provider = "postgresql"
                    url = "{}"
                }}
                "#,
            url
        );

        let me = api.block_on(migration_api(&datamodel)).unwrap();
        api.block_on(me.ensure_connection_validity()).unwrap();
    }

    // Check that the "unexpected" schema now exists.
    {
        let schema_exists_result = api
            .query_raw(
                "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = 'unexpected')",
                &[],
            )
            .unwrap();

        let schema_exists = schema_exists_result
            .into_single()
            .unwrap()
            .at(0)
            .unwrap()
            .as_bool()
            .unwrap();

        assert!(schema_exists)
    }
}
