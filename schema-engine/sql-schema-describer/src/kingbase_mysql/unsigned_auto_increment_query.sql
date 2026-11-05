SELECT
    relation.relname AS table_name,
    attribute.attname AS column_name,
    trigger.tgargs AS trigger_arguments,
    procedure.prosrc AS body
FROM
    sys_catalog.sys_trigger trigger
    JOIN sys_catalog.sys_class relation ON relation.oid = trigger.tgrelid
    JOIN sys_catalog.sys_namespace rn ON rn.oid = relation.relnamespace
    JOIN sys_catalog.sys_proc procedure ON procedure.oid = trigger.tgfoid
    JOIN sys_catalog.sys_namespace pn ON pn.oid = procedure.pronamespace
    JOIN sys_catalog.sys_attribute attribute ON attribute.attrelid = relation.oid
WHERE
    rn.nspname = ?
    AND pn.nspname = rn.nspname
    AND procedure.proname = '__prisma_unsigned_auto_guard'
    AND trigger.tgname = '__prisma_unsigned_auto'
    AND trigger.tgnargs = 2
    AND trigger.tgtype = 21
    AND trigger.tgenabled IN ('O', 'A')
    AND NOT trigger.tgisinternal
    AND attribute.attidentity IN ('a', 'd', 'i')
    AND attribute.atttypid = 'integer' :: regtype
