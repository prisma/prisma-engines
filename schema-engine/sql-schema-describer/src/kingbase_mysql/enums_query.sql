SELECT
    t.typname AS type_name,
    e.enumlabel AS enum_label
FROM
    sys_catalog.sys_type AS t
    INNER JOIN sys_catalog.sys_enum AS e ON e.enumtypid = t.oid
    INNER JOIN sys_catalog.sys_namespace AS n ON n.oid = t.typnamespace
WHERE
    n.nspname = ?
ORDER BY
    t.oid,
    e.enumsortorder
