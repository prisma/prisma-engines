SELECT tbl.relname AS table_name, namespace.nspname as namespace, (tbl.relhassubclass and tbl.relkind = 'p') as is_partition
FROM pg_class AS tbl
INNER JOIN pg_namespace AS namespace ON namespace.oid = tbl.relnamespace
WHERE
  ( -- (relkind = 'r' and relispartition = 't') matches partition table "duplicates"
    (tbl.relkind = 'r' AND tbl.relispartition = 'f')
      OR -- when it's a partition
    tbl.relkind = 'p'
  )
  AND namespace.nspname = ANY ( $1 )
ORDER BY namespace, table_name;
