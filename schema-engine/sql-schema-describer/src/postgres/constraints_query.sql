SELECT
    schemainfo.nspname AS namespace,
    tableinfo.relname AS table_name,
	constr.conname AS constraint_name,
	constr.contype AS constraint_type,
	pg_get_constraintdef(constr.oid) AS constraint_definition,
	constr.condeferrable AS is_deferrable,
	constr.condeferred AS is_deferred
FROM pg_constraint constr
JOIN pg_class AS tableinfo
	ON tableinfo.oid = constr.conrelid
JOIN pg_namespace AS schemainfo
	ON schemainfo.oid = tableinfo.relnamespace
WHERE schemainfo.nspname = ANY ( $1 )
	AND contype NOT IN ('p', 'u', 'f')
ORDER BY namespace, table_name, constr.contype, constraint_name;
