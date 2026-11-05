SELECT
    DISTINCT CAST(o.object_id AS BIGINT) AS object_id,
    n.nspname AS namespace,
    c.relname AS name,
    CAST(o.object_type AS TEXT) AS kind,
    (
        SELECT
            description
        FROM
            sys_catalog.sys_description d
        WHERE
            d.objoid = CAST(CAST(o.object_id AS BIGINT) AS OID)
            AND d.objsubid = 0
            AND d.classoid =(
                SELECT
                    classid
                FROM
                    sys_catalog.sys_get_object_address(
                        'view',
                        ARRAY ['sys_catalog','sys_class'],
                        ARRAY [] :: text []
                    )
            )
    ) AS description,
    CASE
        WHEN o.object_type = 'VIEW' THEN sys_catalog.sys_get_viewdef(CAST(CAST(o.object_id AS BIGINT) AS OID), true)
    END AS definition
FROM
    ALL_OBJECTS o
    JOIN sys_catalog.sys_class c ON c.oid = CAST(CAST(o.object_id AS BIGINT) AS OID)
    JOIN sys_catalog.sys_namespace n ON n.oid = c.relnamespace
WHERE
    n.nspname IN (__SCHEMAS__)
    AND o.object_type IN ('TABLE', 'VIEW')
    AND NOT EXISTS (
        SELECT
            1
        FROM
            sys_catalog.sys_depend d
        WHERE
            d.objid = CAST(CAST(o.object_id AS BIGINT) AS OID)
            AND d.deptype = 'e'
            AND d.classid =(
                SELECT
                    classid
                FROM
                    sys_catalog.sys_get_object_address(
                        'view',
                        ARRAY ['sys_catalog','sys_class'],
                        ARRAY [] :: text []
                    )
            )
    )
ORDER BY
    namespace,
    name
