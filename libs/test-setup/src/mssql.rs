use crate::{Tags, runtime::run_with_thread_local_runtime as tok};
use connection_string::JdbcString;
use enumflags2::BitFlags;
use quaint::{error::Error, prelude::Queryable, single::Quaint};

pub(crate) fn get_mssql_tags(database_url: &str) -> Result<BitFlags<Tags>, String> {
    let fut = async {
        let quaint = Quaint::new(database_url).await.map_err(|err| err.to_string())?;
        let mut tags = Tags::Mssql.into();

        let version = quaint.version().await.map_err(|err| err.to_string())?;

        if let Some(version) = version {
            if version.starts_with("Microsoft SQL Server 2017") {
                tags |= Tags::Mssql2017;
            }

            if version.starts_with("Microsoft SQL Server 2019") {
                tags |= Tags::Mssql2019;
            }
        }

        Ok(tags)
    };

    tok(fut)
}

pub async fn init_mssql_database(original_url: &str, db_name: &str) -> Result<(Quaint, String), Error> {
    let conn = Quaint::new(original_url).await?;
    reset_schema(&conn, db_name).await?;

    let mut url: JdbcString = format!("jdbc:{original_url}").parse().unwrap();
    url.properties_mut().insert("database".into(), db_name.into());
    let url = url.to_string().trim_start_matches("jdbc:").to_owned();

    Ok((conn, url))
}

#[tracing::instrument(skip(conn))]
pub async fn reset_schema(conn: &dyn Queryable, schema_name: &str) -> Result<(), Error> {
    let sql = format!(
        r#"
        DROP DATABASE IF EXISTS [{schema_name}];
        CREATE DATABASE [{schema_name}];
    "#
    );
    conn.raw_cmd(&sql).await?;
    conn.raw_cmd(&format!("USE [{schema_name}];")).await?;

    Ok(())
}
