SELECT
    relation.relname AS table_name,
    attribute.attname AS column_name,
    type_namespace.nspname AS type_namespace,
    t.typname AS type_name,
    e.enumlabel AS enum_label
FROM
    sys_catalog.sys_class AS relation
    INNER JOIN sys_catalog.sys_namespace AS table_namespace ON table_namespace.oid = relation.relnamespace
    INNER JOIN sys_catalog.sys_attribute AS attribute ON attribute.attrelid = relation.oid
    INNER JOIN sys_catalog.sys_type AS t ON t.oid = attribute.atttypid
    INNER JOIN sys_catalog.sys_enum AS e ON e.enumtypid = t.oid
    INNER JOIN sys_catalog.sys_namespace AS type_namespace ON type_namespace.oid = t.typnamespace
WHERE
    BINARY CAST(table_namespace.nspname AS TEXT) = ?
    AND attribute.attnum > 0
    AND NOT attribute.attisdropped
    -- Keep the MySQL-generated ENUM scope; resolve labels through the actual
    -- column type OID, even when that type belongs to another namespace.
    AND LOWER(LEFT(CAST(t.typname AS TEXT), 5)) = 'enum_'
ORDER BY
    relation.oid,
    attribute.attnum,
    e.enumsortorder
