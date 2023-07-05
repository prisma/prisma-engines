SELECT
  tbl.relname AS table_name,
  namespace.nspname as namespace,
  (tbl.relhassubclass and tbl.relkind = 'p') as is_partition,
  (tbl.relhassubclass and tbl.relkind = 'r') as has_subclass,
  tbl.relrowsecurity as has_row_level_security,
  reloptions,
  pd.description as description
FROM pg_class AS tbl
INNER JOIN pg_namespace AS namespace ON namespace.oid = tbl.relnamespace
LEFT JOIN pg_description pd ON pd.objoid = tbl.oid AND pd.objsubid = 0 AND pd.classoid = 'pg_catalog.pg_class'::regclass::oid
WHERE
  ( -- (relkind = 'r' and relispartition = 't') matches partition table "duplicates"
    (tbl.relkind = 'r' AND tbl.relispartition = 'f')
      OR -- when it's a partition
    tbl.relkind = 'p'
  )
  AND namespace.nspname = ANY ( $1 )
ORDER BY namespace, table_name;
