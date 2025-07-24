use crate::{AnyError, Tags, runtime::run_with_thread_local_runtime as tok};
use enumflags2::BitFlags;
use quaint::{prelude::Queryable, single::Quaint};
use url::Url;

pub(crate) fn get_postgres_tags(database_url: &str) -> Result<BitFlags<Tags>, String> {
    let fut = async {
        let quaint = Quaint::new(database_url).await.map_err(|err| err.to_string())?;
        let mut tags = Tags::Postgres.into();
        let version = quaint.version().await.map_err(|err| err.to_string())?;

        match version {
            None => Ok(tags),
            Some(version) => {
                eprintln!("version: {version:?}");

                if version.contains("PostgreSQL 9") {
                    tags |= Tags::Postgres9;
                }

                if version.contains("PostgreSQL 12") {
                    tags |= Tags::Postgres12;
                }

                if version.contains("PostgreSQL 14") {
                    tags |= Tags::Postgres14;
                }

                if version.contains("PostgreSQL 15") {
                    tags |= Tags::Postgres15;
                }

                if version.contains("PostgreSQL 16") {
                    tags |= Tags::Postgres16;
                }

                if version.contains("CockroachDB") {
                    if version.contains("v23.1") {
                        tags |= Tags::CockroachDb231;
                    } else if version.contains("v22.2") {
                        tags |= Tags::CockroachDb222;
                    } else if version.contains("v21.2") {
                        tags |= Tags::CockroachDb221;
                    }

                    tags |= Tags::CockroachDb;
                }

                eprintln!("Inferred tags: {tags:?}");

                Ok(tags)
            }
        }
    };

    tok(fut)
}

pub async fn create_postgres_database(database_url: &str, db_name: &str) -> Result<(Quaint, String), AnyError> {
    let mut url: Url = database_url.parse()?;
    let mut postgres_db_url = url.clone();

    url.set_path(db_name);
    postgres_db_url.set_path("/postgres");

    let drop = format!(
        r#"
        DROP DATABASE IF EXISTS "{db_name}";
        "#,
    );

    let recreate = format!(
        r#"
        CREATE DATABASE "{db_name}";
        "#,
    );

    let conn = Quaint::new(postgres_db_url.as_str()).await?;

    // The two commands have to be run separately on postgres.
    conn.raw_cmd(&drop).await?;
    conn.raw_cmd(&recreate).await?;

    url.query_pairs_mut()
        .append_pair("statement_cache_size", "0")
        .append_pair("schema", "public");

    let url_str = url.to_string();

    let conn = Quaint::new(&url_str).await?;

    conn.raw_cmd("CREATE SCHEMA IF NOT EXISTS \"public\"").await?;

    Ok((conn, url_str))
}
