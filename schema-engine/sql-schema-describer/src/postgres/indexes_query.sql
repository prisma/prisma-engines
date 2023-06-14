WITH rawindex AS (
    SELECT
        indrelid, 
        indexrelid,
        indisunique,
        indisprimary,
        indnkeyatts,
        unnest(indkey) AS indkeyid,
        generate_subscripts(indkey, 1) AS indkeyidx,
        unnest(indclass) AS indclass,
        unnest(indoption) AS indoption
    FROM pg_index
    WHERE
        indpred IS NULL
        AND array_position(indkey::int2[], 0::int2) IS NULL
        AND NOT indisexclusion
),
indexes_info AS (
	SELECT
		schemainfo.nspname AS namespace,
	    indexinfo.relname AS index_name,
	    tableinfo.relname AS table_name,
	    rawindex.indrelid, 
        rawindex.indexrelid,
        rawindex.indnkeyatts,
        rawindex.indkeyidx,
	    columninfo.attname AS column_name,
	    columninfo.attnum
	FROM rawindex
	INNER JOIN pg_class AS tableinfo ON tableinfo.oid = rawindex.indrelid
	INNER JOIN pg_class AS indexinfo ON indexinfo.oid = rawindex.indexrelid
	INNER JOIN pg_namespace AS schemainfo ON schemainfo.oid = tableinfo.relnamespace
	INNER JOIN pg_attribute AS columninfo
	    ON columninfo.attrelid = tableinfo.oid
	    AND columninfo.attnum = rawindex.indkeyid
	INNER JOIN pg_indexes
		ON pg_indexes.schemaname = schemainfo.nspname
		AND pg_indexes.indexname = indexinfo.relname
),
indexes_info_filtered AS (
	SELECT
		namespace,
		index_name,
		table_name,
		indrelid,
		indexrelid,
		indnkeyatts,
		indkeyidx,
		column_name,
		attnum,
		row_num
	FROM (
		SELECT *,
		ROW_NUMBER() OVER (PARTITION BY namespace, index_name, table_name, indrelid, indexrelid, indnkeyatts, indkeyidx ORDER BY attnum) AS row_num
		FROM indexes_info
	) subquery
	WHERE CASE 
        WHEN indnkeyatts = 1 THEN subquery.row_num = 1 AND indkeyidx = 0
        ELSE 1 = 1
    END
)
SELECT DISTINCT
    indexes_info_filtered.namespace,
    indexes_info_filtered.index_name,
    indexes_info_filtered.table_name,
    indexes_info_filtered.column_name,
    rawindex.indisunique AS is_unique,
    rawindex.indisprimary AS is_primary_key,
    indexes_info_filtered.indkeyidx AS column_index,
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
    pc.condeferred AS condeferred
FROM indexes_info_filtered
INNER JOIN pg_class AS tableinfo ON tableinfo.oid = indexes_info_filtered.indrelid
INNER JOIN pg_class AS indexinfo ON indexinfo.oid = indexes_info_filtered.indexrelid
INNER JOIN pg_namespace AS schemainfo ON schemainfo.oid = tableinfo.relnamespace
INNER JOIN rawindex
	ON rawindex.indrelid = indexes_info_filtered.indrelid
	AND rawindex.indexrelid = indexes_info_filtered.indexrelid
	AND rawindex.indkeyidx = indexes_info_filtered.indkeyidx
INNER JOIN pg_attribute AS columninfo
    ON columninfo.attrelid = tableinfo.oid
    AND columninfo.attnum = rawindex.indkeyid
INNER JOIN pg_indexes
	ON pg_indexes.schemaname = schemainfo.nspname
	AND pg_indexes.indexname = indexinfo.relname
INNER JOIN pg_am AS indexaccess ON indexaccess.oid = indexinfo.relam
LEFT JOIN pg_opclass AS opclass -- left join because crdb has no opclasses
    ON opclass.oid = rawindex.indclass
LEFT JOIN pg_constraint pc ON rawindex.indexrelid = pc.conindid AND pc.contype <> 'f'
WHERE schemainfo.nspname = ANY ( $1 )
ORDER BY namespace, table_name, index_name, column_index;
