SELECT
    namespace.nspname AS namespace,
    table_info.relname AS table_name,
    constraint_info.conname AS constraint_name,
    'check' AS constraint_type
-- INFORMATION_SCHEMA also exposes synthetic NOT NULL checks. Read actual
-- CHECK objects instead of guessing their origin from the expression text.
FROM sys_catalog.sys_constraint AS constraint_info
JOIN sys_catalog.sys_class AS table_info ON table_info.oid = constraint_info.conrelid
JOIN sys_catalog.sys_namespace AS namespace ON namespace.oid = table_info.relnamespace
WHERE constraint_info.contype = 'c'
    AND BINARY CAST(namespace.nspname AS TEXT) = ?
ORDER BY namespace, table_name, constraint_type, constraint_name;
