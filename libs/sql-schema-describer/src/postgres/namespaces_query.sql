SELECT namespace.nspname as namespace_name
FROM pg_namespace as namespace
WHERE namespace.nspname = ANY ( $1 )
ORDER BY namespace_name;
