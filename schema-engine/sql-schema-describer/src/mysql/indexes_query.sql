SELECT
    table_schema AS namespace,
    table_name AS table_name,
    index_name AS index_name,
    column_name AS column_name,
    sub_part AS partial,
    seq_in_index AS seq_in_index,
    collation AS column_order,
    non_unique AS non_unique,
    index_type AS index_type
FROM information_schema.statistics
WHERE table_schema IN ({namespaces_filter})
ORDER BY BINARY namespace, BINARY table_name, BINARY index_name, seq_in_index
