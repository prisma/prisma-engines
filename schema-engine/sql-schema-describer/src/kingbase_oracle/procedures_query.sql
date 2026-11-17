SELECT
    DISTINCT n.nspname AS namespace,
    p.proname AS name,
    CASE
        WHEN l.lanname = 'internal' THEN p.prosrc
        ELSE sys_catalog.sys_get_functiondef(p.oid)
    END AS definition
FROM
    ALL_OBJECTS o
    JOIN sys_catalog.sys_proc p ON p.oid = CAST(CAST(o.object_id AS BIGINT) AS OID)
    JOIN sys_catalog.sys_namespace n ON n.oid = p.pronamespace
    JOIN sys_catalog.sys_language l ON l.oid = p.prolang
WHERE
    n.nspname IN (__SCHEMAS__)
    AND o.object_type IN ('FUNCTION', 'PROCEDURE')
ORDER BY
    namespace,
    name
