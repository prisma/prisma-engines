SELECT
    CAST(c.oid AS BIGINT) AS object_id,
    x.relname AS name,
    k.indisprimary AS primary_key,
    k.indisunique AS is_unique,
    pc.conname AS constraint_name,
    a.attname AS column_name,
    CAST(keys.ordinality AS BIGINT) AS ordinal,
    sys_catalog.sys_index_column_has_property(x.oid, CAST(keys.ordinality AS INT), 'desc') AS descending,
    sys_catalog.sys_get_expr(k.indpred, k.indrelid) AS predicate
FROM
    sys_catalog.sys_index k
    JOIN sys_catalog.sys_class c ON c.oid = k.indrelid
    JOIN sys_catalog.sys_namespace n ON n.oid = c.relnamespace
    JOIN sys_catalog.sys_class x ON x.oid = k.indexrelid
    LEFT JOIN sys_catalog.sys_constraint pc ON pc.conindid = k.indexrelid
    AND pc.conrelid = k.indrelid
    AND pc.contype IN ('p', 'u')
    CROSS JOIN LATERAL unnest(k.indkey) WITH ORDINALITY AS keys(attnum, ordinality)
    LEFT JOIN sys_catalog.sys_attribute a ON a.attrelid = c.oid
    AND a.attnum = keys.attnum
WHERE
    n.nspname IN (__SCHEMAS__)
    AND NOT k.indisexclusion
    AND k.indisvalid
    AND keys.ordinality <= k.indnkeyatts
ORDER BY
    n.nspname,
    c.relname,
    x.relname,
    keys.ordinality
