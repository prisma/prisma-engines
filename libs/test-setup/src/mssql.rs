use crate::{runtime::run_with_tokio, Tags};
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

    run_with_tokio(fut)
}

pub async fn init_mssql_database(original_url: &str, db_name: &str) -> Result<(Quaint, String), Error> {
    let mut url: JdbcString = format!("jdbc:{}", original_url).parse().unwrap();
    url.properties_mut().insert("schema".into(), db_name.into());
    let url = url.to_string().trim_start_matches("jdbc:").to_owned();
    let conn = Quaint::new(&url).await?;

    reset_schema(&conn, db_name).await?;

    Ok((conn, url))
}

#[tracing::instrument(skip(conn))]
pub async fn reset_schema(conn: &dyn Queryable, schema_name: &str) -> Result<(), Error> {
    // Mickie misses DROP SCHEMA .. CASCADE, so what we need to do here is to
    // delete first the foreign keys, then all the tables from the test schema
    // to allow a clean slate for the next test.

    let drop_types = format!(
        r#"
        DECLARE @stmt NVARCHAR(max)
        DECLARE @n CHAR(1)

        SET @n = CHAR(10)

        SELECT @stmt = ISNULL(@stmt + @n, '') +
            'DROP TYPE [' + SCHEMA_NAME(schema_id) + '].[' + name + ']'
        FROM sys.types
        WHERE SCHEMA_NAME(schema_id) = '{0}' AND is_user_defined = 1

        EXEC SP_EXECUTESQL @stmt
        "#,
        schema_name
    );

    let drop_procedures = format!(
        r#"
        DECLARE @stmt NVARCHAR(max)
        DECLARE @n CHAR(1)

        SET @n = CHAR(10)

        SELECT @stmt = ISNULL(@stmt + @n, '') +
            'DROP PROCEDURE [' + SCHEMA_NAME(schema_id) + '].[' + OBJECT_NAME(object_id) + ']'
        FROM sys.objects
        WHERE SCHEMA_NAME(schema_id) = '{0}' AND type = 'P'

        EXEC SP_EXECUTESQL @stmt
        "#,
        schema_name
    );

    let drop_shared_defaults = format!(
        r#"
        DECLARE @stmt NVARCHAR(max)
        DECLARE @n CHAR(1)

        SET @n = CHAR(10)

        SELECT @stmt = ISNULL(@stmt + @n, '') +
            'DROP DEFAULT [' + SCHEMA_NAME(schema_id) + '].[' + OBJECT_NAME(object_id) + ']'
        FROM sys.objects
        WHERE SCHEMA_NAME(schema_id) = '{0}' AND type = 'D'

        EXEC SP_EXECUTESQL @stmt
        "#,
        schema_name
    );

    let drop_fks = format!(
        r#"
        DECLARE @stmt NVARCHAR(max)
        DECLARE @n CHAR(1)

        SET @n = CHAR(10)

        SELECT @stmt = ISNULL(@stmt + @n, '') +
            'ALTER TABLE [' + SCHEMA_NAME(schema_id) + '].[' + OBJECT_NAME(parent_object_id) + '] DROP CONSTRAINT [' + name + ']'
        FROM sys.foreign_keys
        WHERE SCHEMA_NAME(schema_id) = '{0}'

        EXEC SP_EXECUTESQL @stmt
        "#,
        schema_name
    );

    let drop_views = format!(
        r#"
        DECLARE @stmt NVARCHAR(max)
        DECLARE @n CHAR(1)

        SET @n = CHAR(10)

        SELECT @stmt = ISNULL(@stmt + @n, '') +
            'DROP VIEW [' + SCHEMA_NAME(schema_id) + '].[' + name + ']'
        FROM sys.views
        WHERE SCHEMA_NAME(schema_id) = '{0}'

        EXEC SP_EXECUTESQL @stmt
        "#,
        schema_name
    );

    let drop_tables = format!(
        r#"
        DECLARE @stmt NVARCHAR(max)
        DECLARE @n CHAR(1)

        SET @n = CHAR(10)

        SELECT @stmt = ISNULL(@stmt + @n, '') +
            'DROP TABLE [' + SCHEMA_NAME(schema_id) + '].[' + name + ']'
        FROM sys.tables
        WHERE SCHEMA_NAME(schema_id) = '{0}'

        EXEC SP_EXECUTESQL @stmt
        "#,
        schema_name
    );

    conn.raw_cmd(&drop_procedures).await?;
    conn.raw_cmd(&drop_views).await?;
    conn.raw_cmd(&drop_fks).await?;
    conn.raw_cmd(&drop_tables).await?;
    conn.raw_cmd(&drop_shared_defaults).await?;
    conn.raw_cmd(&drop_types).await?;

    conn.raw_cmd(&format!("DROP SCHEMA IF EXISTS {}", schema_name)).await?;
    conn.raw_cmd(&format!("CREATE SCHEMA {}", schema_name)).await?;

    Ok(())
}
