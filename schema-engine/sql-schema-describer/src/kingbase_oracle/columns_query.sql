SELECT
    CAST(c.oid AS BIGINT) AS object_id,
    a.attname AS name,
    a.attnum AS ordinal,
    t.typname AS type_name,
    tn.nspname AS type_namespace,
    t.typtype :: text AS type_kind,
    (
        t.typelem <> 0
        AND t.typlen = -1
    ) AS is_array,
    a.attnotnull AS required,
    a.attidentity :: text AS identity,
    i.numeric_precision,
    i.numeric_scale,
    i.character_maximum_length AS length,
    i.datetime_precision AS time_precision,
    sys_catalog.sys_get_expr(d.adbin, d.adrelid) AS default_expression,
    col_description(c.oid, a.attnum) AS description,
    CAST(seq.oid AS BIGINT) AS owned_sequence_oid,
    seq.relname AS owned_sequence_name,
    seqns.nspname AS owned_sequence_namespace,
    EXISTS (
        SELECT
            1
        FROM
            sys_catalog.sys_depend dep
        WHERE
            dep.refobjid = seq.oid
            AND dep.refclassid =(
                SELECT
                    classid
                FROM
                    sys_catalog.sys_get_object_address(
                        'view',
                        ARRAY ['sys_catalog','sys_class'],
                        ARRAY [] :: text []
                    )
            )
            AND dep.deptype = 'n'
            AND dep.objid <> COALESCE(d.oid, 0)
    ) AS sequence_shared
FROM
    ALL_OBJECTS o
    JOIN sys_catalog.sys_class c ON c.oid = CAST(CAST(o.object_id AS BIGINT) AS OID)
    JOIN sys_catalog.sys_namespace n ON n.oid = c.relnamespace
    JOIN sys_catalog.sys_attribute a ON a.attrelid = c.oid
    AND a.attnum > 0
    AND NOT a.attisdropped
    JOIN sys_catalog.sys_type t ON t.oid = a.atttypid
    JOIN sys_catalog.sys_namespace tn ON tn.oid = t.typnamespace
    LEFT JOIN sys_catalog.sys_attrdef d ON d.adrelid = c.oid
    AND d.adnum = a.attnum
    LEFT JOIN sys_catalog.sys_class seq ON seq.oid = CAST(
        sys_catalog.sys_get_serial_sequence(
            quote_ident(n.nspname) || '.' || quote_ident(c.relname),
            a.attname
        ) AS regclass
    )
    LEFT JOIN sys_catalog.sys_namespace seqns ON seqns.oid = seq.relnamespace
    LEFT JOIN information_schema.columns i ON i.table_schema = n.nspname
    AND i.table_name = c.relname
    AND i.column_name = a.attname
WHERE
    n.nspname IN (__SCHEMAS__)
    AND o.object_type IN ('TABLE', 'VIEW')
ORDER BY
    n.nspname,
    c.relname,
    a.attnum
