SELECT tbl.relname AS table_name, namespace.nspname as namespace
FROM pg_class AS tbl
INNER JOIN pg_namespace AS namespace ON namespace.oid = tbl.relnamespace
WHERE tbl.relkind = 'r' AND namespace.nspname = Any ( $1 )
ORDER BY namespace, table_name;
