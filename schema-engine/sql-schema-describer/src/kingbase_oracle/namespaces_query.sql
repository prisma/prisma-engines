SELECT
    n.nspname AS name
FROM
    sys_catalog.sys_namespace n
WHERE
    n.nspname IN (__SCHEMAS__)
ORDER BY
    n.nspname
