WITH rawindex AS (
    SELECT
        indrelid, 
        indexrelid,
        indisunique,
        indisprimary,
        unnest(indkey) AS indkeyid,
        generate_subscripts(indkey, 1) AS indkeyidx,
        unnest(indclass) AS indclass,
        unnest(indoption) AS indoption
    FROM pg_index -- https://www.postgresql.org/docs/current/catalog-pg-index.html
    WHERE
        indpred IS NULL -- filter out partial indexes
        AND array_position(indkey::int2[], 0::int2) IS NULL -- filter out expression indexes
)
SELECT 
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
        AS column_order
FROM
    rawindex
    INNER JOIN pg_class AS tableinfo ON tableinfo.oid = rawindex.indrelid
    INNER JOIN pg_class AS indexinfo ON indexinfo.oid = rawindex.indexrelid
    INNER JOIN pg_namespace AS schemainfo ON schemainfo.oid = tableinfo.relnamespace
    INNER JOIN pg_attribute AS columninfo
        ON columninfo.attrelid = tableinfo.oid AND columninfo.attnum = rawindex.indkeyid
    INNER JOIN pg_am AS indexaccess ON indexaccess.oid = indexinfo.relam
    LEFT JOIN pg_opclass AS opclass -- left join because crdb has no opclasses
        ON opclass.oid = rawindex.indclass
WHERE schemainfo.nspname = Any ( $1 )
ORDER BY schemainfo.nspname, tableinfo.relname, indexinfo.relname, rawindex.indkeyidx;
