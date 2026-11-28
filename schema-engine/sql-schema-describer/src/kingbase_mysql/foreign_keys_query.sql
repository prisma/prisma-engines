SELECT
    constraint_info.conname AS constraint_name,
    column_info.attname AS column_name,
    referenced_namespace.nspname AS referenced_table_schema,
    referenced_table.relname AS referenced_table_name,
    referenced_column.attname AS referenced_column_name,
    keys.ordinality AS ordinal_position,
    table_info.relname AS table_name,
    CASE constraint_info.confdeltype
        WHEN 'a' THEN 'NO ACTION'
        WHEN 'r' THEN 'RESTRICT'
        WHEN 'c' THEN 'CASCADE'
        WHEN 'n' THEN 'SET NULL'
        WHEN 'd' THEN 'SET DEFAULT'
    END AS delete_rule,
    CASE constraint_info.confupdtype
        WHEN 'a' THEN 'NO ACTION'
        WHEN 'r' THEN 'RESTRICT'
        WHEN 'c' THEN 'CASCADE'
        WHEN 'n' THEN 'SET NULL'
        WHEN 'd' THEN 'SET DEFAULT'
    END AS update_rule
FROM sys_catalog.sys_constraint AS constraint_info
    INNER JOIN sys_catalog.sys_class AS table_info ON table_info.oid = constraint_info.conrelid
    INNER JOIN sys_catalog.sys_namespace AS table_namespace ON table_namespace.oid = table_info.relnamespace
    INNER JOIN sys_catalog.sys_class AS referenced_table ON referenced_table.oid = constraint_info.confrelid
    INNER JOIN sys_catalog.sys_namespace AS referenced_namespace ON referenced_namespace.oid = referenced_table.relnamespace
    CROSS JOIN LATERAL unnest(constraint_info.conkey) WITH ORDINALITY AS keys(attnum, ordinality)
    INNER JOIN sys_catalog.sys_attribute AS column_info ON column_info.attrelid = table_info.oid
        AND column_info.attnum = keys.attnum
    INNER JOIN sys_catalog.sys_attribute AS referenced_column ON referenced_column.attrelid = referenced_table.oid
        AND referenced_column.attnum = constraint_info.confkey[keys.ordinality]
WHERE constraint_info.contype = 'f'
    AND BINARY CAST(table_namespace.nspname AS TEXT) = ?
-- R3's referential_constraints.table_name can be a quoted regclass rendering
-- (for example, "User"). Resolve relation and column identities by OID instead.
ORDER BY
    BINARY CAST(table_info.relname AS TEXT),
    BINARY CAST(constraint_info.conname AS TEXT),
    keys.ordinality
