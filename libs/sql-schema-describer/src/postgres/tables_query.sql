SELECT tbl.relname AS table_name, namespace.nspname as namespace, tbl.relhassubclass
FROM pg_class AS tbl
INNER JOIN pg_namespace AS namespace ON namespace.oid = tbl.relnamespace
WHERE
  ( -- grab tables when
    -- it's an oRdinary table ('r') and is not a partition;
    -- NOTE: CockroachDB puts NULLs in 'relispartition'
    (tbl.relkind = 'r' AND ((tbl.relispartition is NULL) OR tbl.relispartition = 'f'))
      OR -- when it's a partition
    tbl.relkind = 'p'
  )
  AND namespace.nspname = ANY ( $1 )
ORDER BY namespace, table_name;
