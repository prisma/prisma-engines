SELECT
    relation.relname AS table_name,
    attribute.attname AS column_name,
    t.typname AS domain_name,
    tn.nspname AS domain_schema,
    sys_catalog.sys_get_expr(co.conbin, 0) AS range_expression
FROM
    sys_catalog.sys_attribute attribute
    JOIN sys_catalog.sys_class relation ON relation.oid = attribute.attrelid
    JOIN sys_catalog.sys_namespace rn ON rn.oid = relation.relnamespace
    JOIN sys_catalog.sys_type t ON t.oid = attribute.atttypid
    JOIN sys_catalog.sys_namespace tn ON tn.oid = t.typnamespace
    JOIN sys_catalog.sys_type base ON base.oid = t.typbasetype
    JOIN sys_catalog.sys_namespace bn ON bn.oid = base.typnamespace
    JOIN sys_catalog.sys_constraint co ON co.contypid = t.oid
WHERE
    rn.nspname = ?
    AND t.typtype = 'd'
    AND NOT t.typnotnull
    AND t.typdefault IS NULL
    AND bn.nspname = 'sys'
    AND base.typname = 'uint4'
    AND co.contype = 'c'
    AND co.convalidated
    AND (
        SELECT
            count(*)
        FROM
            sys_catalog.sys_constraint other
        WHERE
            other.contypid = t.oid
    ) = 1
