SELECT
    n.nspname AS namespace,
    t.typname AS name,
    e.enumlabel AS value,
    (
        SELECT
            description
        FROM
            sys_catalog.sys_description d
        WHERE
            d.objoid = t.oid
            AND d.objsubid = 0
            AND d.classoid =(
                SELECT
                    classid
                FROM
                    sys_catalog.sys_get_object_address('type', ARRAY ['text'], ARRAY [] :: text [])
            )
    ) AS description
FROM
    sys_catalog.sys_type t
    JOIN sys_catalog.sys_namespace n ON n.oid = t.typnamespace
    JOIN sys_catalog.sys_enum e ON e.enumtypid = t.oid
WHERE
    n.nspname IN (__SCHEMAS__)
ORDER BY
    n.nspname,
    t.typname,
    e.enumsortorder
