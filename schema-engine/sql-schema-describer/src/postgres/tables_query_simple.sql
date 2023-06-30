SELECT
  tbl.relname AS table_name,
  namespace.nspname as namespace,
  false as is_partition,
  false as has_subclass,
  false as has_row_level_security,
  reloptions,
  pd.description as description
FROM pg_class AS tbl
INNER JOIN pg_namespace AS namespace ON namespace.oid = tbl.relnamespace
LEFT JOIN pg_description pd ON pd.objoid = tbl.oid AND pd.objsubid = 0 AND pd.classoid = 'pg_catalog.pg_class'::regclass::oid
WHERE
    tbl.relkind = 'r' AND namespace.nspname = ANY ( $1 )
ORDER BY namespace, table_name;
