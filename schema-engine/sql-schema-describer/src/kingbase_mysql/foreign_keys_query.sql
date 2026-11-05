SELECT
    kcu.constraint_name constraint_name,
    kcu.column_name column_name,
    kcu.referenced_table_name referenced_table_name,
    kcu.referenced_column_name referenced_column_name,
    kcu.ordinal_position ordinal_position,
    kcu.table_name table_name,
    rc.delete_rule :: TEXT AS delete_rule,
    rc.update_rule :: TEXT AS update_rule
FROM
    information_schema.key_column_usage AS kcu
    INNER JOIN information_schema.referential_constraints AS rc ON BINARY kcu.constraint_name = BINARY rc.constraint_name
WHERE
    BINARY kcu.table_schema = ?
    AND BINARY rc.constraint_schema = ?
    AND kcu.referenced_column_name IS NOT NULL
ORDER BY
    BINARY kcu.table_schema,
    BINARY kcu.table_name,
    BINARY kcu.constraint_name,
    kcu.ordinal_position
