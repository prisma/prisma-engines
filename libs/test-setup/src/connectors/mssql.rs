use quaint::{error::Error, prelude::Queryable};

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
