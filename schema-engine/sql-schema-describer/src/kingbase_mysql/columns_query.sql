SELECT
    column_name column_name,
    data_type data_type,
    column_type full_data_type,
    character_maximum_length character_maximum_length,
    numeric_precision numeric_precision,
    numeric_scale numeric_scale,
    datetime_precision datetime_precision,
    column_default column_default,
    is_nullable is_nullable,
    CASE
        WHEN extra = 'auto_increment' THEN extra
        WHEN EXISTS (
            SELECT
                1
            FROM
                sys_catalog.sys_attribute AS identity_column
                INNER JOIN sys_catalog.sys_class AS identity_table ON identity_table.oid = identity_column.attrelid
                INNER JOIN sys_catalog.sys_namespace AS identity_schema ON identity_schema.oid = identity_table.relnamespace
            WHERE
                identity_schema.nspname = column_info.table_schema
                AND identity_table.relname = column_info.table_name
                AND identity_column.attname = column_info.column_name
                AND identity_column.attidentity IN ('a', 'd', 'i')
        ) THEN 'auto_increment'
        ELSE extra
    END AS extra,
    table_name table_name,
    IF(column_comment = '', NULL, column_comment) AS column_comment
FROM
    information_schema.columns AS column_info
WHERE
    table_schema = ?
ORDER BY
    ordinal_position
