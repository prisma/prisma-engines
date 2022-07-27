SELECT tbl.relname AS table_name
FROM pg_class AS tbl
INNER JOIN pg_namespace AS namespace ON namespace.oid = tbl.relnamespace
WHERE tbl.relkind = 'r' AND namespace.nspname = $1
ORDER BY table_name;
