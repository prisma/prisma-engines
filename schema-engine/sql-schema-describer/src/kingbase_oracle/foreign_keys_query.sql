SELECT
    CAST(k.oid AS BIGINT) AS constraint_id,
    CAST(k.conrelid AS BIGINT) AS object_id,
    CAST(k.confrelid AS BIGINT) AS referenced_id,
    k.conname AS name,
    sys_catalog.sys_get_constraintdef(k.oid) AS definition,
    a.attname AS column_name,
    ra.attname AS referenced_column,
    rn.nspname AS referenced_namespace,
    rc.relname AS referenced_table,
    CASE
        k.confdeltype
        WHEN 'a' THEN 'NO ACTION'
        WHEN 'r' THEN 'RESTRICT'
        WHEN 'c' THEN 'CASCADE'
        WHEN 'n' THEN 'SET NULL'
        WHEN 'd' THEN 'SET DEFAULT'
    END AS on_delete,
    CASE
        k.confupdtype
        WHEN 'a' THEN 'NO ACTION'
        WHEN 'r' THEN 'RESTRICT'
        WHEN 'c' THEN 'CASCADE'
        WHEN 'n' THEN 'SET NULL'
        WHEN 'd' THEN 'SET DEFAULT'
    END AS on_update
FROM
    sys_catalog.sys_constraint k
    JOIN sys_catalog.sys_class c ON c.oid = k.conrelid
    JOIN sys_catalog.sys_namespace n ON n.oid = c.relnamespace
    JOIN sys_catalog.sys_class rc ON rc.oid = k.confrelid
    JOIN sys_catalog.sys_namespace rn ON rn.oid = rc.relnamespace
    CROSS JOIN LATERAL generate_subscripts(k.conkey, 1) AS keys(ordinal)
    JOIN sys_catalog.sys_attribute a ON a.attrelid = k.conrelid
    AND a.attnum = k.conkey [keys.ordinal]
    JOIN sys_catalog.sys_attribute ra ON ra.attrelid = k.confrelid
    AND ra.attnum = k.confkey [keys.ordinal]
WHERE
    n.nspname IN (__SCHEMAS__)
    AND k.contype = 'f'
ORDER BY
    n.nspname,
    c.relname,
    k.conname,
    keys.ordinal
