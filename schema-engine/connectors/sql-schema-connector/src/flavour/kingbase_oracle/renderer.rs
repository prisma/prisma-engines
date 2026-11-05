use crate::{
    migration_pair::MigrationPair,
    sql_migration::{AlterColumn, AlterEnum, AlterTable, RedefineTable, TableChange},
    sql_renderer::{Quoted, QuotedWithPrefix, SqlRenderer},
};
use itertools::Itertools;
use psl::builtin_connectors::{KingbaseOracleNumberArguments, KingbaseOracleType};
use sql_ddl::{IndexColumn, SortOrder, kingbase_oracle as ddl};
use sql_schema_describer::{
    self as sql, ColumnTypeFamily, DefaultKind, DefaultValue, ForeignKeyAction, PrismaValue, SQLSortOrder, SqlSchema,
    walkers::{
        EnumWalker, ForeignKeyWalker, IndexWalker, TableColumnWalker, TableWalker, UserDefinedTypeWalker, ViewWalker,
    },
};
use std::{borrow::Cow, fmt::Write as _};

/// DDL renderer for KingbaseES in Oracle compatibility mode.
///
/// Oracle-mode public types and `ALTER TABLE ... MODIFY` are emitted here.
/// KES enum evolution is implemented locally, independently of PG version rules.
#[derive(Debug, Default)]
pub(crate) struct KingbaseOracleRenderer;

impl SqlRenderer for KingbaseOracleRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::OracleIdentifier(name)
    }

    fn render_add_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String {
        ddl::AlterTable {
            table_name: &quoted_table(foreign_key.table()),
            clauses: vec![ddl::AlterTableClause::AddForeignKey(ddl::ForeignKey {
                constraint_name: foreign_key.constraint_name().map(Into::into),
                constrained_columns: foreign_key
                    .constrained_columns()
                    .map(|column| column.name().into())
                    .collect(),
                referenced_table: &quoted_table(foreign_key.referenced_table()),
                referenced_columns: foreign_key
                    .referenced_columns()
                    .map(|column| column.name().into())
                    .collect(),
                on_delete: render_foreign_key_action(foreign_key.on_delete_action()),
                on_update: render_foreign_key_action(foreign_key.on_update_action()),
            })],
        }
        .to_string()
    }

    fn render_alter_enum(&self, alter_enum: &AlterEnum, schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        render_oracle_alter_enum(alter_enum, schemas)
    }

    fn render_rename_index(&self, indexes: MigrationPair<IndexWalker<'_>>) -> Vec<String> {
        vec![format!(
            "ALTER INDEX {} RENAME TO {}",
            quoted_index(indexes.previous),
            self.quote(indexes.next.name())
        )]
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        let tables = schemas.walk(alter_table.table_ids);
        let table = quoted_table(tables.previous);
        let mut statements = Vec::new();

        for change in &alter_table.changes {
            match change {
                TableChange::DropPrimaryKey => {
                    let primary_key = tables
                        .previous
                        .primary_key()
                        .expect("drop primary key without a primary key");
                    statements.push(format!(
                        "ALTER TABLE {table} DROP CONSTRAINT {}",
                        self.quote(owning_constraint_name(primary_key))
                    ));
                }
                TableChange::RenamePrimaryKey => {
                    let previous = tables
                        .previous
                        .primary_key()
                        .expect("rename primary key without a primary key");
                    let next = tables
                        .next
                        .primary_key()
                        .expect("rename primary key without a primary key");
                    statements.push(format!(
                        "ALTER TABLE {table} RENAME CONSTRAINT {} TO {}",
                        self.quote(owning_constraint_name(previous)),
                        self.quote(next.name())
                    ));
                }
                TableChange::AddPrimaryKey => {
                    let primary_key = tables
                        .next
                        .primary_key()
                        .expect("add primary key without a primary key");
                    let columns = primary_key.columns().map(|column| self.quote(column.name())).join(", ");
                    statements.push(format!(
                        "ALTER TABLE {table} ADD CONSTRAINT {} PRIMARY KEY ({columns})",
                        self.quote(primary_key.name())
                    ));
                }
                TableChange::AddColumn { column_id, .. } => {
                    let column = tables.next.walk(*column_id);
                    statements.extend(render_add_column(&table, column));
                }
                TableChange::DropColumn { column_id } => {
                    let column = tables.previous.walk(*column_id);
                    statements.push(
                        ddl::AlterTable {
                            table_name: &table,
                            clauses: vec![ddl::AlterTableClause::DropColumn(column.name().into())],
                        }
                        .to_string(),
                    );
                }
                TableChange::AlterColumn(AlterColumn { column_id, changes, .. }) => {
                    let columns = schemas.walk(*column_id);
                    statements.extend(render_alter_column(&table, columns, changes));
                }
                TableChange::DropAndRecreateColumn { column_id, .. } => {
                    let columns = schemas.walk(*column_id);
                    statements.push(
                        ddl::AlterTable {
                            table_name: &table,
                            clauses: vec![ddl::AlterTableClause::DropColumn(columns.previous.name().into())],
                        }
                        .to_string(),
                    );
                    statements.extend(render_add_column(&table, columns.next));
                }
            }
        }

        statements
    }

    fn render_create_enum(&self, create_enum: EnumWalker<'_>) -> Vec<String> {
        vec![create_enum_statement(
            &quoted_name(create_enum.explicit_namespace(), create_enum.name()).to_string(),
            create_enum.values(),
        )]
    }

    fn render_create_index(&self, index: IndexWalker<'_>) -> String {
        assert_ne!(
            index.index_type(),
            sql::IndexType::Fulltext,
            "Kingbase Oracle does not support Prisma full-text indexes"
        );

        // Oracle foreign keys may reference UNIQUE constraints, but not a
        // standalone unique index. Descending indexes cannot back constraints.
        if index.is_unique()
            && !index
                .columns()
                .any(|column| column.sort_order() == Some(SQLSortOrder::Desc))
        {
            return ddl::AlterTable {
                table_name: &quoted_table(index.table()),
                clauses: vec![ddl::AlterTableClause::AddUnique {
                    constraint_name: index.name().into(),
                    columns: index.column_names().map(Into::into).collect(),
                }],
            }
            .to_string();
        }
        ddl::CreateIndex {
            index_name: index.name().into(),
            table_name: &quoted_table(index.table()),
            columns: index
                .columns()
                .map(|column| IndexColumn {
                    name: column.as_column().name().into(),
                    length: None,
                    sort_order: column.sort_order().map(|order| match order {
                        SQLSortOrder::Asc => SortOrder::Asc,
                        SQLSortOrder::Desc => SortOrder::Desc,
                    }),
                    operator_class: None,
                })
                .collect(),
            is_unique: index.is_unique(),
        }
        .to_string()
    }

    fn render_create_table(&self, table: TableWalker<'_>) -> String {
        render_create_table_with_sequences(table, quoted_table(table), table.name())
    }

    fn render_create_table_as(&self, table: TableWalker<'_>, table_name: QuotedWithPrefix<&str>) -> String {
        let escape = |quoted| match quoted {
            #[cfg(feature = "kingbase-mysql")]
            Quoted::KingbaseMysqlIdentifier(name) => Quoted::OracleIdentifier(name),
            Quoted::Double(name)
            | Quoted::Single(name)
            | Quoted::Backticks(name)
            | Quoted::SquareBrackets(name)
            | Quoted::OracleIdentifier(name)
            | Quoted::OracleString(name) => Quoted::OracleIdentifier(name),
        };
        let table_name = QuotedWithPrefix(table_name.0.map(escape), escape(table_name.1));
        render_create_table_with_sequences(table, table_name, table.name())
    }

    fn render_drop_and_recreate_index(&self, indexes: MigrationPair<IndexWalker<'_>>) -> Vec<String> {
        vec![
            self.render_drop_index(None, indexes.previous),
            self.render_create_index(indexes.next),
        ]
    }

    fn render_drop_enum(&self, namespace: Option<&str>, dropped_enum: EnumWalker<'_>) -> Vec<String> {
        let enum_name = quoted_name(namespace, dropped_enum.name());
        vec![format!("DROP TYPE {enum_name}")]
    }

    fn render_drop_foreign_key(&self, _namespace: Option<&str>, foreign_key: ForeignKeyWalker<'_>) -> String {
        format!(
            "ALTER TABLE {} DROP CONSTRAINT IF EXISTS {}",
            quoted_table(foreign_key.table()),
            self.quote(foreign_key.constraint_name().expect("foreign key name is required"))
        )
    }

    fn render_drop_index(&self, namespace: Option<&str>, index: IndexWalker<'_>) -> String {
        use sql::kingbase_oracle::KingbaseOracleSchemaExt;
        let ext: &KingbaseOracleSchemaExt = index.schema.downcast_connector_data();
        if ext.owning_constraint_names.contains_key(&index.id) {
            return format!(
                "ALTER TABLE {} DROP CONSTRAINT {}",
                quoted_table(index.table()),
                self.quote(owning_constraint_name(index))
            );
        }
        let index_name = quoted_name(namespace.or_else(|| index.table().explicit_namespace()), index.name());
        format!("DROP INDEX {index_name}")
    }

    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
        let mut statements = Vec::new();

        for redefine_table in tables {
            let tables = schemas.walk(redefine_table.table_ids);
            let temporary_name = format!("_prisma_new_{}", tables.next.name());
            let temporary_table = quoted_name(tables.next.explicit_namespace(), temporary_name.as_str());
            statements.push(render_create_table_with_sequences(
                tables.next,
                temporary_table,
                &temporary_name,
            ));

            let columns = redefine_table
                .column_pairs
                .iter()
                .map(|(column_ids, _, _)| self.quote(schemas.walk(*column_ids).next.name()).to_string())
                .join(", ");
            if !columns.is_empty() {
                statements.push(format!(
                    "INSERT INTO {temporary_table} ({columns}) SELECT {columns} FROM {}",
                    quoted_table(tables.previous)
                ));
            }

            statements.push(format!("DROP TABLE {} CASCADE", quoted_table(tables.previous)));
            statements.push(self.render_rename_table(
                tables.next.explicit_namespace(),
                &temporary_name,
                tables.next.name(),
            ));

            for index in tables.next.indexes().filter(|index| !index.is_primary_key()) {
                statements.push(self.render_create_index(index));
            }
            for foreign_key in tables.next.foreign_keys() {
                statements.push(self.render_add_foreign_key(foreign_key));
            }
        }

        statements
    }

    fn render_rename_table(&self, namespace: Option<&str>, name: &str, new_name: &str) -> String {
        ddl::AlterTable {
            table_name: &quoted_name(namespace, name),
            clauses: vec![ddl::AlterTableClause::RenameTo(new_name.into())],
        }
        .to_string()
    }

    fn render_drop_view(&self, namespace: Option<&str>, view: ViewWalker<'_>) -> String {
        format!("DROP VIEW {}", quoted_name(namespace, view.name()))
    }

    fn render_drop_user_defined_type(&self, _namespace: Option<&str>, _udt: &UserDefinedTypeWalker<'_>) -> String {
        unreachable!("Kingbase Oracle does not support Prisma extension types")
    }

    fn render_rename_foreign_key(&self, fks: MigrationPair<ForeignKeyWalker<'_>>) -> String {
        format!(
            "ALTER TABLE {} RENAME CONSTRAINT {} TO {}",
            quoted_table(fks.next.table()),
            self.quote(fks.previous.constraint_name().expect("foreign key name is required")),
            self.quote(fks.next.constraint_name().expect("foreign key name is required")),
        )
    }

    fn render_create_namespace(&self, namespace: sql::NamespaceWalker<'_>) -> Vec<String> {
        vec![format!("CREATE SCHEMA IF NOT EXISTS {}", self.quote(namespace.name()))]
    }
}

fn create_enum_statement<'a>(name: &str, values: impl Iterator<Item = &'a str>) -> String {
    ddl::CreateEnum {
        enum_name: &name,
        variants: values.map(Into::into).collect(),
    }
    .to_string()
}

/// Oracle 19c has no schema enum type. Use the documented KES enum interface,
/// including an atomic replacement when labels are removed; no PG version policy.
fn render_oracle_alter_enum(alter: &AlterEnum, schemas: MigrationPair<&SqlSchema>) -> Vec<String> {
    let enums = schemas.walk(alter.id);
    let old_name = quoted_name(enums.previous.explicit_namespace(), enums.previous.name()).to_string();
    // A newly added value cannot be used until ADD VALUE commits. A complete
    // migration is submitted as one script, so defaults in existing/new columns
    // must instead use a replacement type whose labels are immediately usable.
    let default_needs_new_value = schemas.next.walk_table_columns().any(|column| {
        column
            .column_type_family_as_enum()
            .is_some_and(|enm| enm.id == enums.next.id)
            && column.default().is_some_and(|default| match default.kind() {
                DefaultKind::Value(PrismaValue::Enum(value) | PrismaValue::String(value)) => {
                    alter.created_variants.contains(value)
                }
                // Do not guess which labels an arbitrary SQL expression uses.
                DefaultKind::DbGenerated(_) => true,
                _ => false,
            })
    });
    if alter.dropped_variants.is_empty() && !default_needs_new_value {
        return alter
            .created_variants
            .iter()
            .map(|value| format!("ALTER TYPE {old_name} ADD VALUE {}", Quoted::OracleString(value)))
            .collect();
    }
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let new_local = format!("_prisma_enum_new_{suffix}");
    let old_local = format!("_prisma_enum_old_{suffix}");
    let new_name = quoted_name(enums.next.explicit_namespace(), new_local.as_str()).to_string();
    let retired_name = quoted_name(enums.previous.explicit_namespace(), old_local.as_str()).to_string();
    let mut sql = vec![
        "BEGIN".to_owned(),
        create_enum_statement(&new_name, enums.next.values()),
    ];
    // Enum OIDs change during replacement, including on otherwise unchanged
    // FK columns. Release all affected FKs before converting either endpoint.
    let affected_fks: Vec<_> = schemas
        .previous
        .walk_foreign_keys()
        .filter(|fk| {
            fk.constrained_columns().chain(fk.referenced_columns()).any(|column| {
                column
                    .column_type_family_as_enum()
                    .is_some_and(|enm| enm.id == enums.previous.id)
            })
        })
        .collect();
    let renderer = KingbaseOracleRenderer;
    for fk in &affected_fks {
        sql.push(renderer.render_drop_foreign_key(None, *fk));
    }
    for (previous, _) in &alter.previous_usages_as_default {
        let column = schemas.previous.walk(*previous);
        sql.push(format!(
            "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT",
            quoted_table(column.table()),
            Quoted::OracleIdentifier(column.name())
        ));
    }
    // Only existing columns that survive this migration can be converted here.
    for column in schemas.previous.walk_table_columns().filter(|column| {
        column
            .column_type_family_as_enum()
            .is_some_and(|enm| enm.id == enums.previous.id)
    }) {
        let table = column.table();
        let survives = schemas
            .next
            .table_walkers()
            .find(|next| next.name() == table.name() && next.namespace() == table.namespace())
            .and_then(|next| next.column(column.name()))
            .is_some_and(|next| {
                next.column_type_family_as_enum()
                    .is_some_and(|enm| enm.id == enums.next.id)
            });
        if survives {
            sql.push(format!(
                "ALTER TABLE {} ALTER COLUMN {} TYPE {new_name} USING (CAST(CAST({} AS TEXT) AS {new_name}))",
                quoted_table(table),
                Quoted::OracleIdentifier(column.name()),
                Quoted::OracleIdentifier(column.name())
            ));
        } else {
            sql.push(format!(
                "ALTER TABLE {} ALTER COLUMN {} TYPE TEXT USING (CAST({} AS TEXT))",
                quoted_table(table),
                Quoted::OracleIdentifier(column.name()),
                Quoted::OracleIdentifier(column.name())
            ));
        }
    }
    sql.push(format!(
        "ALTER TYPE {old_name} RENAME TO {}",
        Quoted::OracleIdentifier(old_local.as_str())
    ));
    sql.push(format!(
        "ALTER TYPE {new_name} RENAME TO {}",
        Quoted::OracleIdentifier(enums.next.name())
    ));
    sql.push(format!("DROP TYPE {retired_name}"));
    for (previous, next) in &alter.previous_usages_as_default {
        if let Some(next) = next.and_then(|id| schemas.next.walk(id).default()) {
            let column = schemas.previous.walk(*previous);
            sql.push(format!(
                "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {}",
                quoted_table(column.table()),
                Quoted::OracleIdentifier(column.name()),
                render_default(next.inner())
            ));
        }
    }
    // Restore the previous constraint when both endpoint columns still use
    // this enum. Ordinary migration steps can subsequently rename, replace or
    // remove it. Retiring columns must not regain incompatible constraints.
    for fk in affected_fks {
        let columns_survive = fk.constrained_columns().chain(fk.referenced_columns()).all(|column| {
            schemas
                .next
                .table_walkers()
                .find(|t| t.name() == column.table().name() && t.namespace() == column.table().namespace())
                .and_then(|t| t.column(column.name()))
                .is_some_and(|next| {
                    if column
                        .column_type_family_as_enum()
                        .is_some_and(|enm| enm.id == enums.previous.id)
                    {
                        next.column_type_family_as_enum()
                            .is_some_and(|enm| enm.id == enums.next.id)
                    } else {
                        true
                    }
                })
        });
        if columns_survive {
            let ext: &sql::kingbase_oracle::KingbaseOracleSchemaExt = schemas.previous.downcast_connector_data();
            sql.push(match ext.foreign_key_definitions.get(&fk.id) {
                Some(definition) => format!(
                    "ALTER TABLE {} ADD CONSTRAINT {} {definition}",
                    quoted_table(fk.table()),
                    Quoted::OracleIdentifier(fk.constraint_name().expect("foreign key name"))
                ),
                None => renderer.render_add_foreign_key(fk),
            });
        }
    }
    sql.push("COMMIT".to_owned());
    sql
}

fn render_alter_column(
    table: &QuotedWithPrefix<&str>,
    columns: MigrationPair<TableColumnWalker<'_>>,
    changes: &crate::sql_schema_differ::ColumnChanges,
) -> Vec<String> {
    let column_name = Quoted::OracleIdentifier(columns.previous.name());
    let mut statements = Vec::new();
    let ext: &sql::kingbase_oracle::KingbaseOracleSchemaExt = columns.previous.schema.downcast_connector_data();
    let generation = ext.column_generations.get(&columns.previous.id);
    let removing_identity = changes.autoincrement_changed()
        && !columns.next.is_autoincrement()
        && matches!(
            generation,
            Some(sql::kingbase_oracle::ColumnGeneration::Identity { .. })
        );
    if removing_identity {
        if let Some(sql::kingbase_oracle::ColumnGeneration::Identity {
            sequence: Some(sequence),
        }) = generation
        {
            statements.push(render_remove_identity(table, &column_name, sequence));
        } else {
            statements.push(format!("ALTER TABLE {table} ALTER COLUMN {column_name} DROP IDENTITY"));
        }
    }

    if changes.type_changed() {
        statements.push(format!(
            "ALTER TABLE {table} MODIFY ({column_name} {})",
            render_column_type(columns.next)
        ));
    }

    if changes.default_changed() {
        match columns.next.default() {
            Some(default) if !matches!(default.kind(), DefaultKind::DbGenerated(None)) => statements.push(format!(
                "ALTER TABLE {table} ALTER COLUMN {column_name} SET DEFAULT {}",
                render_default(default.inner())
            )),
            _ => statements.push(format!("ALTER TABLE {table} ALTER COLUMN {column_name} DROP DEFAULT")),
        }
    }

    if changes.arity_changed() {
        let nullability = if columns.next.arity().is_required() {
            "NOT NULL"
        } else {
            "NULL"
        };
        statements.push(format!("ALTER TABLE {table} MODIFY ({column_name} {nullability})"));
    }

    if changes.autoincrement_changed() {
        if columns.next.is_autoincrement() {
            let sequence_name = quoted_sequence(columns.next, columns.next.table().name());
            // Keep calibration and installing the default atomic for schema
            // push as well as script execution. The table lock prevents writes
            // between reading MAX and enabling the generated default.
            let commands = [
                format!("LOCK TABLE {table} IN ACCESS EXCLUSIVE MODE"),
                format!("CREATE SEQUENCE {sequence_name}"),
                format!(
                    "SELECT sys_catalog.setval({}, CAST(GREATEST(COALESCE(MAX(CAST({column_name} AS NUMERIC)), 0) + 1, 1) AS BIGINT), false) FROM {table}",
                    Quoted::OracleString(&sequence_name)
                ),
                format!(
                    "ALTER TABLE {table} ALTER COLUMN {column_name} SET DEFAULT nextval({})",
                    Quoted::OracleString(&sequence_name)
                ),
                format!("ALTER SEQUENCE {sequence_name} OWNED BY {table}.{column_name}"),
            ];
            let body = format!(
                "BEGIN\n{}\nEND;",
                commands
                    .iter()
                    .map(|sql| format!("EXECUTE {};", Quoted::OracleString(sql)))
                    .join("\n")
            );
            statements.push(format!("DO {}", Quoted::OracleString(&body)));
        } else {
            // A simultaneous SET DEFAULT (e.g. autoincrement() -> 3) above
            // must survive removing the old sequence.
            if !changes.default_changed() && !removing_identity {
                statements.push(format!("ALTER TABLE {table} ALTER COLUMN {column_name} DROP DEFAULT"));
            }
            if let Some(sql::kingbase_oracle::ColumnGeneration::OwnedSequence {
                namespace,
                name,
                shared,
            }) = generation
            {
                let sequence = quoted_name(Some(namespace), name);
                statements.push(if *shared {
                    format!("ALTER SEQUENCE {sequence} OWNED BY NONE")
                } else {
                    format!("DROP SEQUENCE {sequence}")
                });
            }
        }
    }

    statements
}

/// One atomic KES DO statement: preserve a shared sequence under its original
/// name, then rebind current defaults before DROP IDENTITY deletes the old OID.
/// Read dependencies at execution time: earlier migration steps may already
/// have changed or removed a referencing column/default.
fn render_remove_identity(
    table: &QuotedWithPrefix<&str>,
    column: &Quoted<&str>,
    sequence: &sql::kingbase_oracle::IdentitySequence,
) -> String {
    let original = quoted_name(Some(&sequence.namespace), &sequence.name).to_string();
    let retired_local = format!("_prisma_identity_{}", uuid::Uuid::new_v4().simple());
    let retired = quoted_name(Some(&sequence.namespace), &retired_local).to_string();
    let defaults_query = format!(
        r#"
SELECT COALESCE(jsonb_agg(jsonb_build_object('namespace',n.nspname,'table',c.relname,
    'column',a.attname,'expression',sys_catalog.sys_get_expr(d.adbin,d.adrelid)) ORDER BY n.nspname,c.relname,a.attnum),'[]'::jsonb)
INTO defaults FROM sys_catalog.sys_depend dep JOIN sys_catalog.sys_attrdef d ON d.oid=dep.objid
JOIN sys_catalog.sys_class c ON c.oid=d.adrelid JOIN sys_catalog.sys_namespace n ON n.oid=c.relnamespace
JOIN sys_catalog.sys_attribute a ON a.attrelid=c.oid AND a.attnum=d.adnum
WHERE dep.refobjid={}::regclass AND dep.deptype='n'
AND dep.classid=(SELECT classid FROM sys_catalog.sys_get_object_address('default value',ARRAY[n.nspname,c.relname,a.attname],ARRAY[]::text[]))
AND dep.refclassid=(SELECT classid FROM sys_catalog.sys_get_object_address('view',ARRAY['sys_catalog','sys_class'],ARRAY[]::text[]));
"#,
        Quoted::OracleString(&original)
    );
    let rename = format!(
        "ALTER SEQUENCE {original} RENAME TO {}",
        Quoted::OracleIdentifier(&retired_local)
    );
    let create = format!("CREATE SEQUENCE {original} {}", sequence.options);
    let copy = format!(
        "SELECT sys_catalog.setval({},last_value,is_called) FROM {retired}",
        Quoted::OracleString(&original)
    );
    let drop = format!("ALTER TABLE {table} ALTER COLUMN {column} DROP IDENTITY");
    let lock = format!("LOCK TABLE {table} IN ACCESS EXCLUSIVE MODE");
    let body = format!(
        r#"DECLARE defaults jsonb; reference record; BEGIN
EXECUTE {lock};
{defaults_query}
FOR reference IN SELECT value AS item FROM jsonb_array_elements(defaults) LOOP
    EXECUTE format('LOCK TABLE %I.%I IN ACCESS EXCLUSIVE MODE',reference.item->>'namespace',reference.item->>'table');
END LOOP;
{defaults_query}
IF jsonb_array_length(defaults)>0 THEN
    EXECUTE {rename}; EXECUTE {create}; EXECUTE {copy};
    FOR reference IN SELECT value AS item FROM jsonb_array_elements(defaults) LOOP
        EXECUTE format('ALTER TABLE %I.%I ALTER COLUMN %I SET DEFAULT %s',reference.item->>'namespace',reference.item->>'table',reference.item->>'column',reference.item->>'expression');
    END LOOP;
END IF;
EXECUTE {drop}; END;"#,
        lock = Quoted::OracleString(&lock),
        rename = Quoted::OracleString(&rename),
        create = Quoted::OracleString(&create),
        copy = Quoted::OracleString(&copy),
        drop = Quoted::OracleString(&drop)
    );
    format!("DO {}", Quoted::OracleString(&body))
}

fn render_create_table_with_sequences(
    table: TableWalker<'_>,
    table_name: QuotedWithPrefix<&str>,
    sequence_table_name: &str,
) -> String {
    let create_table = ddl::CreateTable {
        table_name: &table_name,
        columns: table
            .columns()
            .map(|column| render_column(column, sequence_table_name))
            .collect(),
        primary_key: table.primary_key().map(|primary_key| ddl::PrimaryKey {
            constraint_name: primary_key.name().into(),
            columns: primary_key.columns().map(|column| column.name().into()).collect(),
        }),
    }
    .to_string();
    let mut statements = table
        .columns()
        .filter(|column| column.is_autoincrement())
        .map(|column| render_create_sequence(column, sequence_table_name))
        .collect::<Vec<_>>();

    statements.push(create_table);
    statements.extend(
        table
            .columns()
            .filter(|column| column.is_autoincrement())
            .map(|column| render_own_sequence(column, &table_name, sequence_table_name)),
    );

    statements.join(";\n")
}

fn render_add_column(table: &QuotedWithPrefix<&str>, column: TableColumnWalker<'_>) -> Vec<String> {
    let mut statements = Vec::new();

    if column.is_autoincrement() {
        statements.push(render_create_sequence(column, column.table().name()));
    }

    statements.push(
        ddl::AlterTable {
            table_name: table,
            clauses: vec![ddl::AlterTableClause::AddColumn(render_column(
                column,
                column.table().name(),
            ))],
        }
        .to_string(),
    );

    if column.is_autoincrement() {
        statements.push(render_own_sequence(column, table, column.table().name()));
    }

    statements
}

fn render_column<'a>(column: TableColumnWalker<'a>, sequence_table_name: &str) -> ddl::Column<'a> {
    let default = if column.is_autoincrement() {
        let sequence = quoted_sequence(column, sequence_table_name);
        Some(format!("nextval({})", ddl::OracleString(&sequence)).into())
    } else {
        column
            .default()
            .filter(|default| {
                !matches!(
                    default.kind(),
                    DefaultKind::DbGenerated(None) | DefaultKind::Sequence(_)
                )
            })
            .map(|default| render_default(default.inner()))
    };
    ddl::Column {
        name: column.name().into(),
        r#type: render_column_type(column),
        not_null: column.arity().is_required(),
        default,
    }
}

fn render_column_type(column: TableColumnWalker<'_>) -> Cow<'static, str> {
    if let Some(enm) = column.column_type_family_as_enum() {
        return quoted_name(enm.explicit_namespace(), enm.name()).to_string().into();
    }

    if let ColumnTypeFamily::Unsupported(description) = &column.column_type().family {
        return description.to_owned().into();
    }

    let native_type = column
        .column_native_type::<KingbaseOracleType>()
        .expect("missing Kingbase Oracle native type in renderer");

    optional_argument_type(native_type)
}

fn optional_argument_type(native_type: &KingbaseOracleType) -> Cow<'static, str> {
    fn optional_argument(prefix: &str, argument: Option<u32>) -> Cow<'static, str> {
        match argument {
            Some(argument) => format!("{prefix}({argument})").into(),
            None => prefix.to_owned().into(),
        }
    }

    match native_type {
        KingbaseOracleType::Number(KingbaseOracleNumberArguments::Unspecified) => "NUMBER".into(),
        KingbaseOracleType::Number(KingbaseOracleNumberArguments::Precision(precision)) => {
            format!("NUMBER({precision})").into()
        }
        KingbaseOracleType::Number(KingbaseOracleNumberArguments::PrecisionAndScale(precision, scale)) => {
            format!("NUMBER({precision},{scale})").into()
        }
        KingbaseOracleType::TinyInt => "TINYINT".into(),
        KingbaseOracleType::Float(precision) => optional_argument("FLOAT", *precision),
        KingbaseOracleType::BinaryFloat => "BINARY_FLOAT".into(),
        KingbaseOracleType::BinaryDouble => "BINARY_DOUBLE".into(),
        KingbaseOracleType::Char(length) => optional_argument("CHAR", *length),
        KingbaseOracleType::VarChar2(length) => optional_argument("VARCHAR2", *length),
        KingbaseOracleType::NChar(length) => optional_argument("NCHAR", *length),
        KingbaseOracleType::NVarChar2(length) => optional_argument("NVARCHAR2", *length),
        KingbaseOracleType::Clob => "CLOB".into(),
        KingbaseOracleType::NClob => "NCLOB".into(),
        KingbaseOracleType::Blob => "BLOB".into(),
        KingbaseOracleType::Date => "DATE".into(),
        KingbaseOracleType::Timestamp(precision) => optional_argument("TIMESTAMP", *precision),
        KingbaseOracleType::TimestampTz(precision) => match precision {
            Some(precision) => format!("TIMESTAMP({precision}) WITH TIME ZONE").into(),
            None => "TIMESTAMP WITH TIME ZONE".into(),
        },
        KingbaseOracleType::TimestampLocalTz(precision) => match precision {
            Some(precision) => format!("TIMESTAMP({precision}) WITH LOCAL TIME ZONE").into(),
            None => "TIMESTAMP WITH LOCAL TIME ZONE".into(),
        },
        KingbaseOracleType::Boolean => "BOOLEAN".into(),
        KingbaseOracleType::Json => "JSON".into(),
        KingbaseOracleType::Uuid => "UUID".into(),
        KingbaseOracleType::Xml => "XML".into(),
    }
}

fn render_default(default: &DefaultValue) -> Cow<'_, str> {
    match default.kind() {
        DefaultKind::DbGenerated(Some(value)) => value.as_str().into(),
        DefaultKind::DbGenerated(None) | DefaultKind::Sequence(_) => Cow::Borrowed(""),
        DefaultKind::Now => Cow::Borrowed("CURRENT_TIMESTAMP"),
        DefaultKind::UniqueRowid => unreachable!("Kingbase Oracle does not support unique_rowid()"),
        DefaultKind::Value(PrismaValue::String(value) | PrismaValue::Enum(value) | PrismaValue::Json(value)) => {
            format!("'{}'", value.replace('\'', "''")).into()
        }
        DefaultKind::Value(PrismaValue::DateTime(value)) => Quoted::OracleString(value).to_string().into(),
        DefaultKind::Value(PrismaValue::Bytes(value)) => {
            let mut encoded = String::with_capacity(value.len() * 2);
            for byte in value {
                write!(encoded, "{byte:02x}").unwrap();
            }
            format!("decoding({}, 'hex')", Quoted::OracleString(&encoded)).into()
        }
        DefaultKind::Value(value) => value.to_string().into(),
    }
}

fn render_foreign_key_action(action: ForeignKeyAction) -> ddl::ForeignKeyAction {
    match action {
        ForeignKeyAction::Cascade => ddl::ForeignKeyAction::Cascade,
        ForeignKeyAction::NoAction => ddl::ForeignKeyAction::NoAction,
        ForeignKeyAction::Restrict => ddl::ForeignKeyAction::Restrict,
        ForeignKeyAction::SetDefault => ddl::ForeignKeyAction::SetDefault,
        ForeignKeyAction::SetNull => ddl::ForeignKeyAction::SetNull,
    }
}

fn quoted_name<'a>(namespace: Option<&'a str>, name: &'a str) -> QuotedWithPrefix<&'a str> {
    QuotedWithPrefix(namespace.map(Quoted::OracleIdentifier), Quoted::OracleIdentifier(name))
}

fn quoted_table(table: TableWalker<'_>) -> QuotedWithPrefix<&str> {
    quoted_name(table.explicit_namespace(), table.name())
}

fn quoted_index(index: IndexWalker<'_>) -> QuotedWithPrefix<&str> {
    quoted_name(index.table().explicit_namespace(), index.name())
}

fn owning_constraint_name(index: IndexWalker<'_>) -> &str {
    let ext: &sql::kingbase_oracle::KingbaseOracleSchemaExt = index.schema.downcast_connector_data();
    ext.owning_constraint_names
        .get(&index.id)
        .map(String::as_str)
        .unwrap_or_else(|| index.name())
}

pub(super) fn sequence_name(table_name: &str, column_name: &str) -> String {
    let name = format!("{table_name}_{column_name}_seq");
    if name.len() <= 63 && name == name.to_lowercase() {
        return name;
    }
    let hash = name.bytes().fold(0xcbf29ce484222325_u64, |state, byte| {
        (state ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    let mut prefix = name.to_lowercase();
    while prefix.len() > 40 {
        prefix.pop();
    }
    format!("{prefix}_{hash:016x}")
}

fn quoted_sequence(column: TableColumnWalker<'_>, table_name: &str) -> String {
    let sequence_name = sequence_name(table_name, column.name());
    quoted_name(column.table().explicit_namespace(), &sequence_name).to_string()
}

fn render_create_sequence(column: TableColumnWalker<'_>, table_name: &str) -> String {
    format!("CREATE SEQUENCE {}", quoted_sequence(column, table_name))
}

fn render_own_sequence(
    column: TableColumnWalker<'_>,
    table_name: &QuotedWithPrefix<&str>,
    sequence_table_name: &str,
) -> String {
    format!(
        "ALTER SEQUENCE {} OWNED BY {table_name}.{}",
        quoted_sequence(column, sequence_table_name),
        Quoted::OracleIdentifier(column.name())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ddl_layout_preserves_migration_scripts() {
        use psl::parser_database::NoExtensionTypes;
        use schema_connector::SchemaDialect;

        let dialect = crate::SqlSchemaDialect::kingbase_oracle();
        let datasource = "datasource db {\n provider = \"kingbase-oracle\"\n}\n";
        let models = r#"
enum Status {
  READY
  DONE
}
model Parent {
  id Int @id @default(autoincrement())
  name String @default("O'Reilly\\path") @map("na\"me")
  state Status @default(READY)
  children Child[]
  @@map("Pa\"rent")
  @@index([name(sort: Desc)], map: "name\"index")
}
model Child {
  id Int @id
  parentId Int
  parent Parent @relation(fields: [parentId], references: [id], onDelete: Cascade, map: "fk\"parent")
  code String? @unique(map: "code\"unique")
}
"#;
        let source = format!("{datasource}{models}");
        let calculate = |source: String| {
            dialect
                .schema_from_datamodel(
                    vec![("schema.prisma".into(), source.into())],
                    Some("public"),
                    &NoExtensionTypes,
                )
                .unwrap()
        };
        let target = source
            .replace("@default(\"O'Reilly\\\\path\")", "@default(\"changed\")")
            .replace("code String?", "code String");
        let create = dialect.diff(
            calculate(datasource.into()),
            calculate(source.clone()),
            &Default::default(),
        );
        let alter = dialect.diff(calculate(source), calculate(target), &Default::default());
        let create =
            crate::apply_migration::render_script(&create, &Default::default(), &KingbaseOracleRenderer).unwrap();
        let alter =
            crate::apply_migration::render_script(&alter, &Default::default(), &KingbaseOracleRenderer).unwrap();
        let scripts = format!("{create}\n{alter}");
        expect_test::expect![[r#"
-- CreateEnum
CREATE TYPE "Status" AS ENUM ('READY', 'DONE');

-- CreateTable
CREATE SEQUENCE "pa""rent_id_seq_16d279fb84efd911";
CREATE TABLE "Pa""rent" (
    "id" NUMBER(10,0) NOT NULL DEFAULT nextval('"pa""rent_id_seq_16d279fb84efd911"'),
    "na""me" VARCHAR2(4000) NOT NULL DEFAULT 'O''Reilly\path',
    "state" "Status" NOT NULL DEFAULT 'READY',
    CONSTRAINT "Pa""rent_pkey" PRIMARY KEY ("id")
);
ALTER SEQUENCE "pa""rent_id_seq_16d279fb84efd911" OWNED BY "Pa""rent"."id";

-- CreateTable
CREATE TABLE "Child" (
    "id" NUMBER(10,0) NOT NULL,
    "parentId" NUMBER(10,0) NOT NULL,
    "code" VARCHAR2(4000),
    CONSTRAINT "Child_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "name""index" ON "Pa""rent" ("na""me" DESC);

-- CreateIndex
ALTER TABLE "Child" ADD CONSTRAINT "code""unique" UNIQUE ("code");

-- AddForeignKey
ALTER TABLE "Child" ADD CONSTRAINT "fk""parent" FOREIGN KEY ("parentId") REFERENCES "Pa""rent" ("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AlterTable
ALTER TABLE "Pa""rent" ALTER COLUMN "na""me" SET DEFAULT 'changed';

-- AlterTable
ALTER TABLE "Child" MODIFY ("code" NOT NULL);
"#]].assert_eq(&scripts);
    }

    #[test]
    fn drops_the_actual_owning_constraint_instead_of_the_index_name() {
        use sql::kingbase_oracle::KingbaseOracleSchemaExt;
        let mut schema = SqlSchema::default();
        let table = schema.push_table("items".into(), Default::default(), None);
        let owned = schema.push_unique_constraint(table, "supporting_index".into());
        let standalone = schema.push_unique_constraint(table, "standalone_unique".into());
        let mut ext = KingbaseOracleSchemaExt::default();
        ext.owning_constraint_names.insert(owned, "actual_constraint".into());
        schema.set_connector_data(Box::new(ext));
        let renderer = KingbaseOracleRenderer;
        assert_eq!(
            renderer.render_drop_index(None, schema.walk(owned)),
            "ALTER TABLE \"items\" DROP CONSTRAINT \"actual_constraint\""
        );
        assert_eq!(
            renderer.render_drop_index(None, schema.walk(standalone)),
            "DROP INDEX \"standalone_unique\""
        );
    }

    #[test]
    fn multiple_enum_additions_do_not_inherit_postgresql_version_advice() {
        let mut previous = SqlSchema::default();
        let ns = previous.push_namespace("public".into());
        let old = previous.push_enum(ns, "State".into(), None);
        previous.push_enum_variant(old, "old".into());
        let mut next = previous.clone();
        next.push_enum_variant(old, "new1".into());
        next.push_enum_variant(old, "new2".into());
        let alter = AlterEnum {
            id: MigrationPair::new(old, old),
            created_variants: vec!["new1".into(), "new2".into()],
            dropped_variants: vec![],
            previous_usages_as_default: vec![],
        };
        let statements = KingbaseOracleRenderer.render_alter_enum(&alter, MigrationPair::new(&previous, &next));
        assert_eq!(statements.len(), 2);
        assert!(
            statements
                .iter()
                .all(|s| s.starts_with("ALTER TYPE ") && !s.contains("PostgreSQL"))
        );
    }

    #[test]
    fn renders_oracle_native_type_names() {
        assert_eq!(
            optional_argument_type(&KingbaseOracleType::TimestampTz(Some(3))),
            "TIMESTAMP(3) WITH TIME ZONE"
        );
        assert_eq!(
            optional_argument_type(&KingbaseOracleType::Number(
                KingbaseOracleNumberArguments::PrecisionAndScale(10, 2)
            )),
            "NUMBER(10,2)"
        );
    }

    #[test]
    fn renders_binary_defaults_as_decoding_calls() {
        assert_eq!(
            render_default(&DefaultValue::value(PrismaValue::Bytes(vec![0xab, 0xcd]))),
            "decoding('abcd', 'hex')"
        );
    }

    #[test]
    fn string_defaults_escape_quotes_without_changing_backslashes() {
        assert_eq!(
            render_default(&DefaultValue::value(PrismaValue::String("'potassium'\\path".into()))),
            "'''potassium''\\path'"
        );
    }
    #[test]
    fn rebuilding_an_index_without_an_explicit_namespace_still_qualifies_it() {
        let mut schema = SqlSchema::default();
        let namespace = schema.push_namespace("tenant".into());
        let table = schema.push_table("items".into(), namespace, None);
        let index = schema.push_index(table, "items_name".into());
        schema.set_connector_data(Box::new(sql::kingbase_oracle::KingbaseOracleSchemaExt::default()));
        assert_eq!(
            KingbaseOracleRenderer.render_drop_index(None, schema.walk(index)),
            "DROP INDEX \"tenant\".\"items_name\""
        );
    }
}
