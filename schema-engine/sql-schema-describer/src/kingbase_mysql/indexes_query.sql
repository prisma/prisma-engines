SELECT
    tableinfo.relname AS table_name,
    indexinfo.relname AS index_name,
    indexdescription.description AS index_description,
    columninfo.attname AS column_name,
    NULL :: INT AS partial,
    keys.ordinality :: INT AS seq_in_index,
    CASE
        WHEN sys_catalog.sys_index_column_has_property(
            rawindex.indexrelid,
            keys.ordinality :: INT,
            'desc'
        ) THEN 'D'
        WHEN sys_catalog.sys_index_column_has_property(rawindex.indexrelid, keys.ordinality :: INT, 'asc') THEN 'A'
        ELSE NULL
    END AS column_order,
    NOT rawindex.indisunique AS non_unique,
    indexaccess.amname AS index_type,
    rawindex.indisprimary AS is_primary,
    indexconstraint.conname AS constraint_name,
    sys_catalog.sys_get_expr(rawindex.indexprs, rawindex.indrelid) AS index_expression
FROM
    sys_catalog.sys_index AS rawindex
    INNER JOIN sys_catalog.sys_class AS tableinfo ON tableinfo.oid = rawindex.indrelid
    INNER JOIN sys_catalog.sys_class AS indexinfo ON indexinfo.oid = rawindex.indexrelid
    LEFT JOIN sys_catalog.sys_description AS indexdescription ON indexdescription.objoid = indexinfo.oid
    AND indexdescription.objsubid = 0
    AND indexdescription.classoid = (
        SELECT
            classid
        FROM
            sys_catalog.sys_get_object_address(
                'view',
                ARRAY ['sys_catalog', 'sys_class'],
                ARRAY [] :: text []
            )
    )
    INNER JOIN sys_catalog.sys_namespace AS schemainfo ON schemainfo.oid = tableinfo.relnamespace
    INNER JOIN sys_catalog.sys_am AS indexaccess ON indexaccess.oid = indexinfo.relam
    LEFT JOIN sys_catalog.sys_constraint AS indexconstraint ON indexconstraint.conindid = rawindex.indexrelid
    AND indexconstraint.conrelid = rawindex.indrelid
    AND indexconstraint.contype IN ('p', 'u')
    CROSS JOIN LATERAL unnest(rawindex.indkey) WITH ORDINALITY AS keys(attnum, ordinality)
    LEFT JOIN sys_catalog.sys_attribute AS columninfo ON columninfo.attrelid = tableinfo.oid
    AND columninfo.attnum = keys.attnum
WHERE
    schemainfo.nspname = ?
    AND NOT rawindex.indisexclusion
    AND keys.ordinality <= rawindex.indnkeyatts
ORDER BY
    table_name,
    index_name,
    seq_in_index
