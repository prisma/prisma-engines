WITH cte_constraints AS (
	SELECT
		constr.oid AS constraint_id,
		constr.conname AS constraint_name,
		constr.contype AS constraint_type,
		pg_get_constraintdef(constr.oid) AS constraint_definition
	FROM pg_constraint constr
	JOIN pg_class AS tableinfo
		ON tableinfo.oid = constr.conrelid
	JOIN pg_namespace AS ns
		ON ns.oid = tableinfo.relnamespace
	WHERE
		ns.nspname = ANY ( $1 )
		AND constr.contype IN ('f', 'x', 'c')
),
cte_constraints_agg_json AS (
	SELECT json_build_object(
		'check', (
			SELECT json_agg(json_build_object(
				'namespace', ns.nspname,
				'table_name', cl.relname,
				'constraint_name', constr.conname,
				'constraint_definition', pg_get_constraintdef(constr.oid),
				'is_deferrable', constr.condeferrable,
				'is_deferred', constr.condeferred
			))
			FROM cte_constraints ctec
			JOIN pg_constraint constr
				ON constr.oid = ctec.constraint_id
			JOIN pg_class cl
				ON constr.conrelid = cl.oid
			JOIN pg_namespace AS ns
				ON ns.oid = cl.relnamespace
			WHERE contype = 'c'
		),
	  'exclusion', (
			SELECT json_agg(json_build_object(
				'namespace', ns.nspname,
				'table_name', cl.relname,
				'constraint_name', constr.conname,
				'constraint_definition', pg_get_constraintdef(constr.oid),
				'is_deferrable', constr.condeferrable,
				'is_deferred', constr.condeferred
			))
			FROM cte_constraints ctec
			JOIN pg_constraint constr
				ON constr.oid = ctec.constraint_id
			JOIN pg_class cl
				ON constr.conrelid = cl.oid
			JOIN pg_namespace AS ns
				ON ns.oid = cl.relnamespace
			WHERE contype = 'x'
		),
		'foreign_key', (
			SELECT json_agg(json_build_object(
				'con_id', con.oid,
				'child_column', att2.attname,
				'parent_table', cl.relname,
				'parent_column', att.attname,
				'confdeltype', con.confdeltype,
				'confupdtype', con.confupdtype,
				'referenced_schema_name', rel_ns.nspname,
				'constraint_name', conname,
				'child', child,
				'parent', parent,
				'table_name', table_name,
				'namespace', namespace
			) ORDER BY con.namespace, table_name, conname, con.oid, con.colidx)
			FROM (
				SELECT
					ns.nspname AS "namespace",
					con1.contype,
					unnest(con1.conkey)                AS "parent",
					unnest(con1.confkey)                AS "child",
					cl.relname                          AS table_name,
					ns.nspname                          AS schema_name,
					generate_subscripts(con1.conkey, 1) AS colidx,
					con1.oid,
					con1.confrelid,
					con1.conrelid,
					con1.conname,
					con1.confdeltype,
					con1.confupdtype
				FROM cte_constraints ctec
				JOIN pg_constraint con1
					ON con1.oid = ctec.constraint_id
				JOIN pg_class cl
					ON con1.conrelid = cl.oid
				JOIN pg_namespace ns
					ON cl.relnamespace = ns.oid
				WHERE con1.contype IN ('f', 'x', 'c')
				ORDER BY colidx
			) con
			JOIN pg_attribute att on att.attrelid = con.confrelid and att.attnum = con.child
			JOIN pg_class cl on cl.oid = con.confrelid
			JOIN pg_attribute att2 on att2.attrelid = con.conrelid and att2.attnum = con.parent
			JOIN pg_class rel_cl on con.confrelid = rel_cl.oid
			JOIN pg_namespace rel_ns on rel_cl.relnamespace = rel_ns.oid
		)
	) as constraints
)
SELECT
	COALESCE(constraints->>'check', '[]') "check",
	COALESCE(constraints->>'exclusion', '[]') "exclusion",
	COALESCE(constraints->>'foreign_key', '[]') "foreign_key"
FROM cte_constraints_agg_json;
