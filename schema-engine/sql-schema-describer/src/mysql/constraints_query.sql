SELECT
	tc.table_schema AS namespace,
	tc.table_name AS table_name,
	tc.constraint_name AS constraint_name,
	LOWER(tc.constraint_type) AS constraint_type,
	cc.check_clause AS constraint_definition
FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc
LEFT JOIN INFORMATION_SCHEMA.CHECK_CONSTRAINTS cc
	ON cc.constraint_schema = tc.table_schema
	AND cc.constraint_name = tc.constraint_name
WHERE constraint_type = 'CHECK'
ORDER BY BINARY namespace, BINARY table_name, constraint_type, constraint_name;
