SELECT
    CAST(k.conrelid AS BIGINT) AS object_id,
    k.conname AS name
FROM
    sys_catalog.sys_constraint k
    JOIN sys_catalog.sys_class c ON c.oid = k.conrelid
    JOIN sys_catalog.sys_namespace n ON n.oid = c.relnamespace
WHERE
    n.nspname IN (__SCHEMAS__)
    AND k.contype = 'c'
ORDER BY
    k.conrelid,
    k.conname
