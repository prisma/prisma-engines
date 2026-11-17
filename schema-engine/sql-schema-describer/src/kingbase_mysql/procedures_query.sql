SELECT
    routine_name AS name,
    routine_definition AS definition
FROM
    information_schema.routines
WHERE
    ROUTINE_SCHEMA = ?
    AND ROUTINE_TYPE = 'PROCEDURE'
