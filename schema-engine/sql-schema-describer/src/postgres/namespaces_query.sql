SELECT namespace.nspname as namespace_name
FROM pg_namespace as namespace
WHERE namespace.nspname = ANY ( $1 )
AND namespace.nspname <> 'crdb_internal'
ORDER BY namespace_name;
