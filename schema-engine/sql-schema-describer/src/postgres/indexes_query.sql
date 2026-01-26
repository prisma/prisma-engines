WITH rawindex AS (
    SELECT
        indrelid, 
        indexrelid,
        indisunique,
        indisprimary,
        indpred,
        unnest(indkey) AS indkeyid,
        generate_subscripts(indkey, 1) AS indkeyidx,
        unnest(indclass) AS indclass,
        unnest(indoption) AS indoption
    FROM pg_index -- https://www.postgresql.org/docs/current/catalog-pg-index.html
    WHERE
        NOT indisexclusion -- filter out exclusion constraints
)
SELECT
    schemainfo.nspname AS namespace,
    indexinfo.relname AS index_name,
    tableinfo.relname AS table_name,
    columninfo.attname AS column_name,
    rawindex.indisunique AS is_unique,
    rawindex.indisprimary AS is_primary_key,
    rawindex.indkeyidx AS column_index,
    opclass.opcname AS opclass,
    opclass.opcdefault AS opcdefault,
    indexaccess.amname AS index_algo,
    CASE rawindex.indoption & 1
        WHEN 1 THEN 'DESC'
        ELSE 'ASC' END
        AS column_order,
    CASE rawindex.indoption & 2
        WHEN 2 THEN true
        ELSE false END
        AS nulls_first,
    pc.condeferrable AS condeferrable,
    pc.condeferred AS condeferred,
    pg_get_expr(rawindex.indpred, rawindex.indrelid) AS predicate
FROM
    rawindex
    INNER JOIN pg_class AS tableinfo ON tableinfo.oid = rawindex.indrelid
    INNER JOIN pg_class AS indexinfo ON indexinfo.oid = rawindex.indexrelid
    INNER JOIN pg_namespace AS schemainfo ON schemainfo.oid = tableinfo.relnamespace
    LEFT JOIN pg_attribute AS columninfo
        ON columninfo.attrelid = tableinfo.oid AND columninfo.attnum = rawindex.indkeyid
    INNER JOIN pg_am AS indexaccess ON indexaccess.oid = indexinfo.relam
    LEFT JOIN pg_opclass AS opclass -- left join because crdb has no opclasses
        ON opclass.oid = rawindex.indclass
    LEFT JOIN pg_constraint pc ON rawindex.indexrelid = pc.conindid AND pc.contype <> 'f'
WHERE schemainfo.nspname = ANY ( $1 )
ORDER BY namespace, table_name, index_name, column_index;
