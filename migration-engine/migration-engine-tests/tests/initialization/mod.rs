use migration_core::{migration_api, GateKeeper};
use migration_engine_tests::postgres_10_url;
use quaint::prelude::Queryable;
use url::Url;

#[tokio::test]
async fn connecting_to_a_postgres_database_with_missing_schema_creates_it() {
    let url_str = postgres_10_url("test_connecting_with_a_nonexisting_schema");
    test_setup::create_postgres_database(&url_str.parse().unwrap())
        .await
        .unwrap();
    let conn = quaint::single::Quaint::new(&url_str).await.unwrap();

    // Check that the "unexpected" schema does not exist.
    {
        let schema_exists_result = conn
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
        let mut url: Url = url_str.parse().unwrap();

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

        migration_api(&datamodel, GateKeeper::allow_all_whitelist())
            .await
            .unwrap();
    }

    // Check that the "unexpected" schema now exists.
    {
        let schema_exists_result = conn
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
}
