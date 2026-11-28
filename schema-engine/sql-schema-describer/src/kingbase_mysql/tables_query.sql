SELECT
    DISTINCT BINARY table_info.table_name AS table_name,
    table_info.create_options AS create_options,
    table_info.table_comment AS table_comment
FROM
    information_schema.tables AS table_info
    JOIN information_schema.columns AS column_info ON BINARY column_info.table_name = BINARY table_info.table_name
WHERE
    table_info.table_schema = ?
    AND column_info.table_schema = ?
    -- Exclude views.
    AND table_info.table_type = 'BASE TABLE'
ORDER BY
    BINARY table_info.table_name
