use migration_core::migration_api;
use migration_engine_tests::{multi_engine_test_api::*, TestResult};
use quaint::prelude::Queryable;
use test_macros::test_connector;
use url::Url;

#[test_connector(tags(Postgres))]
async fn connecting_to_a_postgres_database_with_missing_schema_creates_it(api: &TestApi) -> TestResult {
    // let url_str = postgres_10_url("test_connecting_with_a_nonexisting_schema").0;
    // test_setup::create_postgres_database(&url_str.parse().unwrap())
    //     .await
    //     .unwrap();
    // let conn = quaint::single::Quaint::new(&url_str).await.unwrap();

    // Check that the "unexpected" schema does not exist.
    {
        let schema_exists_result = api
            .admin_conn()
            .query_raw(
                "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = 'unexpected')",
                &[],
            )
            .await
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

        migration_api(&datamodel).await.unwrap();
    }

    // Check that the "unexpected" schema now exists.
    {
        let schema_exists_result = api
            .admin_conn()
            .query_raw(
                "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = 'unexpected')",
                &[],
            )
            .await
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

    Ok(())
}
