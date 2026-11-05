SELECT
    t.typname,
    CAST(s.seqstart AS BIGINT) AS seqstart,
    CAST(s.seqincrement AS BIGINT) AS seqincrement,
    CAST(s.seqmin AS BIGINT) AS seqmin,
    CAST(s.seqmax AS BIGINT) AS seqmax,
    CAST(s.seqcache AS BIGINT) AS seqcache,
    s.seqcycle
FROM
    sys_catalog.sys_sequence s
    JOIN sys_catalog.sys_type t ON t.oid = s.seqtypid
WHERE
    s.seqrelid = CAST(CAST($1 AS BIGINT) AS OID)
