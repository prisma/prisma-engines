SELECT
    views.TABLE_NAME AS view_name,
    views.VIEW_DEFINITION AS view_sql
FROM
    INFORMATION_SCHEMA.VIEWS AS views
    INNER JOIN sys_catalog.sys_namespace AS namespace ON namespace.nspname = views.TABLE_SCHEMA
    INNER JOIN sys_catalog.sys_class AS relation ON relation.relnamespace = namespace.oid
    AND relation.relname = views.TABLE_NAME
WHERE
    views.TABLE_SCHEMA = ?
    -- Do not expose views installed by database extensions as user views.
    AND NOT EXISTS (
        SELECT
            1
        FROM
            sys_catalog.sys_depend AS dependency
            -- sys_class is a view; its own OID is not the relation classid.
        WHERE
            dependency.classid = (
                SELECT
                    classid
                FROM
                    sys_catalog.sys_get_object_address(
                        'view',
                        ARRAY ['sys_catalog', 'sys_class'],
                        ARRAY [] :: text []
                    )
            )
            AND dependency.objid = relation.oid
            AND dependency.deptype = 'e'
    );
