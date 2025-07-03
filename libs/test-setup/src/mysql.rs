use crate::{runtime::run_with_thread_local_runtime as tok, AnyError, Tags};
use enumflags2::BitFlags;
use quaint::{prelude::Queryable, single::Quaint};
use url::Url;

/// The maximum length of identifiers on mysql is 64 bytes.
///
/// Source: https://dev.mysql.com/doc/mysql-reslimits-excerpt/5.5/en/identifier-length.html
pub fn mysql_safe_identifier(identifier: &str) -> &str {
    if identifier.len() <= 64 {
        identifier
    } else {
        identifier.get(0..64).expect("mysql identifier truncation")
    }
}

pub(crate) fn get_mysql_tags(database_url: &str) -> Result<BitFlags<Tags>, String> {
    let fut = async {
        let quaint = Quaint::new(database_url).await.map_err(|err| err.to_string())?;
        let mut tags: BitFlags<Tags> = Tags::Mysql.into();

        let metadata = quaint
            .query_raw(
                "SELECT @@lower_case_table_names lower_cases_table_names, @@version version",
                &[],
            )
            .await
            .map_err(|err| err.to_string())?;

        let first_row = metadata
            .first()
            .ok_or_else(|| "Got an empty result set when fetching metadata".to_owned())?;

        if let Some(1) = first_row
            .get("lower_cases_table_names")
            .and_then(|lctn| lctn.as_integer())
        {
            tags |= Tags::LowerCasesTableNames;
        }

        match first_row.get("version").and_then(|version| version.to_string()) {
            None => Ok(tags),
            Some(version) => {
                eprintln!("Version: {version:?}");

                // order matters...

                if version.contains("MariaDB") {
                    tags |= Tags::Mariadb
                } else {
                    if version.contains("vitess") {
                        tags |= Tags::Vitess;
                    }

                    if version.contains("5.6") {
                        tags |= Tags::Mysql56
                    }

                    if version.contains("5.7") {
                        tags |= Tags::Mysql57
                    }

                    if version.contains("8.") {
                        tags |= Tags::Mysql8
                    }
                }

                eprintln!("Inferred tags: {tags:?}");

                Ok(tags)
            }
        }
    };

    tok(fut)
}

/// Returns a connection to the new database, as well as the corresponding
/// complete connection string.
#[allow(clippy::needless_lifetimes)] // clippy is wrong
pub async fn create_mysql_database<'a>(database_url: &str, db_name: &'a str) -> Result<(&'a str, String), AnyError> {
    let mut url: Url = database_url.parse()?;
    let mut mysql_db_url = url.clone();
    let db_name = mysql_safe_identifier(db_name);

    mysql_db_url.set_path("/mysql");
    url.set_path(db_name);

    debug_assert!(!db_name.is_empty());
    debug_assert!(
        db_name.len() <= 64,
        "db_name should be at most 64 characters, got {:?}",
        db_name.len()
    );

    let conn = Quaint::new(mysql_db_url.as_ref()).await?;

    let drop = format!(
        r#"
        DROP DATABASE IF EXISTS `{db_name}`;
        "#,
    );

    let recreate = format!(
        r#"
        CREATE DATABASE `{db_name}`;
        "#,
    );

    // The two commands have to be run separately on mariadb.
    conn.raw_cmd(&drop).await?;
    conn.raw_cmd(&recreate).await?;
    let url_str = url.to_string();

    Ok((db_name, url_str))
}

/// Drops the given namespaces (aka "databases" in mysql) from the database without recreating them..
pub async fn drop_mysql_namespaces(database_url: &str, namespaces: &[&str]) -> Result<(), AnyError> {
    let url: Url = database_url.parse()?;
    let conn: Quaint = Quaint::new(url.as_ref()).await?;

    for namespace in namespaces {
        let db_name = mysql_safe_identifier(namespace);
        let drop = format!(
            r#"            
            SET FOREIGN_KEY_CHECKS=0;
            DROP DATABASE IF EXISTS `{db_name}`;
            SET FOREIGN_KEY_CHECKS=1;
        "#,
        );
        conn.raw_cmd(&drop).await?;
    }

    Ok(())
}
