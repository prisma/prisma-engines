WITH rawindex AS (
    SELECT
        tableinfo.oid,
        schemainfo.nspname AS namespace,
	    indexinfo.relname AS index_name,
	    tableinfo.relname AS table_name,
        indrelid, 
        indexrelid,
        indisunique,
        indisprimary,
        -- `indnkeyatts` was introduced in Postgres 11.
        -- It's the number of key columns in the index, not counting any included columns
        CASE
        	WHEN has_indnkeyatts
        	THEN indnkeyatts::text::int2
        	ELSE tableinfo.relnatts
        END AS indnkeyatts,
        -- `indnatts` was introduced in Postgres 11.
        -- It's the total number of columns in the index
        CASE
        	WHEN has_indnatts
        	THEN indnatts::text::int2
        	ELSE tableinfo.relnatts
        END AS indnatts,
        unnest(indkey) AS indkeyid,
        generate_subscripts(indkey, 1) AS indkeyidx,
        unnest(indclass) AS indclass,
        unnest(indoption) AS indoption,
        pg_get_expr(indexprs, indrelid) AS index_expression
    FROM pg_index -- https://www.postgresql.org/docs/current/catalog-pg-index.html
 	INNER JOIN pg_class AS tableinfo ON tableinfo.oid = pg_index.indrelid
	INNER JOIN pg_class AS indexinfo ON indexinfo.oid = pg_index.indexrelid
	INNER JOIN pg_namespace AS schemainfo ON schemainfo.oid = tableinfo.relnamespace
	INNER JOIN pg_indexes
		ON pg_indexes.schemaname = schemainfo.nspname
		AND pg_indexes.indexname = indexinfo.relname
    -- Provide `pg_catalog.pg_index.indnkeyatts` if available
    CROSS JOIN (
	   SELECT EXISTS (
	        SELECT FROM information_schema.columns 
	        WHERE table_schema = 'pg_catalog'
		  	    AND table_name = 'pg_index'
		        AND column_name = 'indnkeyatts'
	        ) AS has_indnkeyatts
	   ) indnkeyatts
	-- Provide `pg_catalog.pg_index.indnatts` if available
    CROSS JOIN (
	   SELECT EXISTS (
	        SELECT FROM information_schema.columns 
	        WHERE table_schema = 'pg_catalog'
	      	    AND table_name = 'pg_index'
	      	    AND column_name = 'indnatts'
	        ) AS has_indnatts
	   ) indnatts
    WHERE
        indpred IS NULL -- filter out partial indexes
        AND NOT indisexclusion -- filter out exclusion constraints
),
indexes_info AS (
	SELECT
		rawindex.namespace,
	    rawindex.index_name,
	    rawindex.table_name,
	    rawindex.indrelid, 
        rawindex.indexrelid,
        rawindex.indnkeyatts,
        rawindex.indnatts,
        rawindex.indkeyidx,
	    columninfo.attname AS column_name,
	    columninfo.attnum,
        rawindex.index_expression
	FROM rawindex
    -- You may wonder, why `LEFT JOIN` here?
    -- Expression Indexes are generally defined without `column_info` - they do not refer to a specific column, as they contain an expression.
    -- Due to this, we need to update the query to handle indexes where column_info is nullable, otherwise we lose expression indexes in our result set.
	LEFT JOIN pg_attribute AS columninfo
	    ON columninfo.attrelid = rawindex.oid
	    AND columninfo.attnum = rawindex.indkeyid
),
indexes_info_filtered AS (
	SELECT
		namespace,
		index_name,
		table_name,
		indrelid,
		indexrelid,
		indnkeyatts,
        indnatts,
		indkeyidx,
		column_name,
		attnum,
		row_num,
        index_expression
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
    CASE
        WHEN indexes_info_filtered.indnatts > indexes_info_filtered.indnkeyatts THEN 'INCLUDE'
        WHEN indexes_info_filtered.index_expression IS NOT NULL THEN 'EXPRESSION'
        ELSE 'REGULAR'
    END AS index_type,
    indexes_info_filtered.column_name, -- NULL in the case of expression indexes
    indexes_info_filtered.index_expression, -- NULL unless `index_type == 'expression'`
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
-- You may wonder, why `LEFT JOIN` here? Same reason as above.
LEFT JOIN pg_attribute AS columninfo
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
