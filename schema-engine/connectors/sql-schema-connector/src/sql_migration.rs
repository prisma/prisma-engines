use crate::{
    migration_pair::MigrationPair,
    sql_renderer::IteratorJoin,
    sql_schema_differ::{ColumnChange, ColumnChanges},
};
use enumflags2::BitFlags;
use sql_schema_describer::{
    EnumId, ForeignKeyId, IndexId, SqlSchema, TableColumnId, TableId, UdtId, ViewId,
    postgres::{self, PostgresSchemaExt},
    walkers::{TableColumnWalker, TableWalker},
};
use std::{collections::BTreeSet, fmt::Write as _};

/// The database migration type for SqlMigrationConnector.
#[derive(Debug)]
pub struct SqlMigration {
    pub(crate) before: SqlSchema,
    pub(crate) after: SqlSchema,
    pub(crate) steps: Vec<SqlMigrationStep>,
}

impl SqlMigration {
    pub(crate) fn schemas(&self) -> MigrationPair<&SqlSchema> {
        MigrationPair::new(&self.before, &self.after)
    }

    /// Exposed for tests.
    ///
    /// Rendering of the drift summary proceeds in two steps:
    ///
    /// - For each step, compute a _prefix_ (DriftItem, &str) containing the
    ///   _type_ of change it is (so we can order between added tables and
    ///   changed enums, for example), and then in which section of the summary
    ///   they appear, when relevant (e.g. changed tables).
    /// - Based on the computed sections and their ordering, we render each
    ///   block in the summary one by one.
    pub fn drift_summary(&self) -> String {
        if self.steps.is_empty() {
            return "No difference detected.".to_owned();
        }

        // The order of the variants matters
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
        #[repr(u8)]
        enum DriftType {
            AlteredExtension,
            DroppedExtension,
            CreatedExtension,
            AddedEnum,
            AddedTable,
            RemovedEnum,
            RemovedTable,
            RemovedUdt,
            RemovedView,
            RedefinedTable,
            ChangedEnum,
            ChangedTable,
        }

        // (sort key, item name, step index)
        let mut drift_items: BTreeSet<(DriftType, &str, u32)> = BTreeSet::new();

        for (idx, step) in self.steps.iter().enumerate() {
            let idx = idx as u32;
            match step {
                SqlMigrationStep::AlterSequence(_, _) => (),
                SqlMigrationStep::CreateSchema(_) => (), // todo
                SqlMigrationStep::DropView(drop_view) => {
                    drift_items.insert((
                        DriftType::RemovedView,
                        self.schemas().previous.walk(drop_view.view_id).name(),
                        idx,
                    ));
                }
                SqlMigrationStep::DropUserDefinedType(drop_udt) => {
                    drift_items.insert((
                        DriftType::RemovedUdt,
                        self.schemas().previous.walk(drop_udt.udt_id).name(),
                        idx,
                    ));
                }
                SqlMigrationStep::CreateEnum(_) => {
                    drift_items.insert((DriftType::AddedEnum, "", idx));
                }
                SqlMigrationStep::AlterEnum(alter_enum) => {
                    drift_items.insert((
                        DriftType::ChangedEnum,
                        self.schemas().walk(alter_enum.id).previous.name(),
                        idx,
                    ));
                }
                SqlMigrationStep::DropForeignKey { foreign_key_id } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().previous.walk(*foreign_key_id).table().name(),
                        idx,
                    ));
                }
                SqlMigrationStep::AlterPrimaryKey(table_id) => {
                    drift_items.insert((DriftType::ChangedTable, self.before.walk(table_id.previous).name(), idx));
                }
                SqlMigrationStep::DropIndex { index_id } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().previous.walk(*index_id).table().name(),
                        idx,
                    ));
                }
                SqlMigrationStep::AlterTable(alter_table) => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().walk(alter_table.table_ids).previous.name(),
                        idx,
                    ));
                }
                SqlMigrationStep::DropTable { .. } => {
                    drift_items.insert((DriftType::RemovedTable, "", idx));
                }
                SqlMigrationStep::DropEnum(_) => {
                    drift_items.insert((DriftType::RemovedEnum, "", idx));
                }
                SqlMigrationStep::CreateTable { .. } => {
                    drift_items.insert((DriftType::AddedTable, "", idx));
                }
                SqlMigrationStep::RedefineTables(redefines) => {
                    for redefine in redefines {
                        drift_items.insert((
                            DriftType::RedefinedTable,
                            self.schemas().walk(redefine.table_ids).previous.name(),
                            idx,
                        ));
                    }
                }
                SqlMigrationStep::RenameForeignKey { foreign_key_id } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().walk(*foreign_key_id).next.table().name(),
                        idx,
                    ));
                }
                SqlMigrationStep::CreateIndex {
                    table_id: (_, table_id),
                    ..
                } => {
                    drift_items.insert((DriftType::ChangedTable, self.schemas().next.walk(*table_id).name(), idx));
                }
                SqlMigrationStep::AddForeignKey { foreign_key_id: id } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().next.walk(*id).table().name(),
                        idx,
                    ));
                }
                SqlMigrationStep::RenameIndex { index } | SqlMigrationStep::RedefineIndex { index } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().walk(*index).previous.table().name(),
                        idx,
                    ));
                }
                SqlMigrationStep::CreateExtension(create_extension) => {
                    let ext: &PostgresSchemaExt = self.schemas().next.downcast_connector_data();
                    let extension = ext.get_extension(create_extension.id);

                    drift_items.insert((DriftType::CreatedExtension, &extension.name, idx));
                }
                SqlMigrationStep::AlterExtension(alter_extension) => {
                    let ext: &PostgresSchemaExt = self.schemas().previous.downcast_connector_data();
                    let extension = ext.get_extension(alter_extension.ids.previous);

                    drift_items.insert((DriftType::AlteredExtension, &extension.name, idx));
                }
                SqlMigrationStep::DropExtension(drop_extension) => {
                    let ext: &PostgresSchemaExt = self.schemas().previous.downcast_connector_data();
                    let extension = ext.get_extension(drop_extension.id);

                    drift_items.insert((DriftType::DroppedExtension, &extension.name, idx));
                }
            };
        }

        let mut out = String::with_capacity(self.steps.len() * 20);
        let mut render_state = (DriftType::AddedEnum, "");

        for (line_idx, (new_state, item_name, step_idx)) in drift_items.iter().enumerate() {
            if render_state != (*new_state, item_name) || line_idx == 0 {
                match new_state {
                    DriftType::AddedEnum => {
                        out.push_str("\n[+] Added enums\n");
                    }
                    DriftType::AddedTable => {
                        out.push_str("\n[+] Added tables\n");
                    }
                    DriftType::RemovedEnum => out.push_str("\n[-] Removed enums\n"),
                    DriftType::RemovedTable => out.push_str("\n[-] Removed tables\n"),
                    DriftType::RemovedUdt => out.push_str("\n[-] Removed UDTs\n"),
                    DriftType::RemovedView => out.push_str("\n[-] Removed views\n"),
                    DriftType::RedefinedTable => {
                        out.push_str("\n[*] Redefined table `");
                        out.push_str(item_name);
                        out.push_str("`\n")
                    }
                    DriftType::ChangedEnum => {
                        out.push_str("\n[*] Changed the `");
                        out.push_str(item_name);
                        out.push_str("` enum\n");
                    }
                    DriftType::ChangedTable => {
                        out.push_str("\n[*] Changed the `");
                        out.push_str(item_name);
                        out.push_str("` table\n");
                    }
                    DriftType::CreatedExtension => {
                        out.push_str("\n[+] Added extensions\n");
                    }
                    DriftType::AlteredExtension => {
                        out.push_str("\n[*] Changed the `");
                        out.push_str(item_name);
                        out.push_str("` extension\n");
                    }
                    DriftType::DroppedExtension => {
                        out.push_str("\n[-] Removed extensions\n`");
                    }
                }
            }

            render_state = (*new_state, *item_name);

            match &self.steps[*step_idx as usize] {
                SqlMigrationStep::AlterSequence(_, _) => {}
                SqlMigrationStep::DropView(_) => {}
                SqlMigrationStep::DropUserDefinedType(_) => {}
                SqlMigrationStep::CreateEnum(enum_id) => {
                    out.push_str("  - ");
                    out.push_str(self.schemas().next.walk(*enum_id).name());
                    out.push('\n');
                }
                SqlMigrationStep::CreateSchema(_) => {} // todo
                SqlMigrationStep::AlterEnum(alter_enum) => {
                    for added in &alter_enum.created_variants {
                        out.push_str("  [+] Added variant `");
                        out.push_str(added);
                        out.push_str("`\n");
                    }

                    for dropped in &alter_enum.dropped_variants {
                        out.push_str("  [-] Removed variant `");
                        out.push_str(dropped);
                        out.push_str("`\n");
                    }
                }
                SqlMigrationStep::AlterPrimaryKey(table_id) => {
                    let table_name = self.schemas().previous.walk(table_id.previous).name();
                    out.push_str("   [*] Changed the primary key for `");
                    out.push_str(table_name);
                    out.push_str("`\n");
                }
                SqlMigrationStep::DropForeignKey { foreign_key_id } => {
                    let fk = self.schemas().previous.walk(*foreign_key_id);

                    out.push_str("  [-] Removed foreign key on columns (");
                    out.push_str(&fk.constrained_columns().map(|c| c.name()).join(", "));
                    out.push_str(")\n")
                }
                SqlMigrationStep::DropIndex { index_id } => {
                    let index = self.schemas().previous.walk(*index_id);

                    out.push_str("  [-] Removed ");

                    if index.is_unique() {
                        out.push_str("unique ");
                    }

                    out.push_str("index on columns (");
                    out.push_str(&index.column_names().join(", "));
                    out.push_str(")\n");
                }
                SqlMigrationStep::AlterTable(alter_table) => {
                    let tables = self.schemas().walk(alter_table.table_ids);

                    for change in &alter_table.changes {
                        match change {
                            TableChange::AddColumn {
                                column_id,
                                has_virtual_default: _,
                            } => {
                                out.push_str("  [+] Added column `");
                                out.push_str(self.schemas().next.walk(*column_id).name());
                                out.push_str("`\n");
                            }
                            TableChange::AlterColumn(alter_column) => {
                                out.push_str("  [*] Altered column `");
                                write!(
                                    out,
                                    "{}` ",
                                    self.schemas().next.walk(alter_column.column_id.next).name(),
                                )
                                .unwrap();
                                render_column_changes(
                                    self.schemas().walk(alter_column.column_id),
                                    &alter_column.changes,
                                    &mut out,
                                );
                                out.push('\n');
                            }
                            TableChange::DropColumn { column_id } => {
                                out.push_str("  [-] Removed column `");
                                out.push_str(self.schemas().previous.walk(*column_id).name());
                                out.push_str("`\n");
                            }
                            TableChange::DropAndRecreateColumn { column_id, changes } => {
                                out.push_str("  [*] Column `");
                                out.push_str(self.schemas().next.walk(column_id.next).name());
                                out.push_str("` would be dropped and recreated ");
                                render_column_changes(self.schemas().walk(*column_id), changes, &mut out);
                                out.push('\n');
                            }
                            TableChange::DropPrimaryKey => {
                                out.push_str("  [-] Dropped the primary key on columns (");
                                render_primary_key_column_names(tables.previous, &mut out);
                                out.push_str(")\n");
                            }
                            TableChange::RenamePrimaryKey => {
                                out.push_str("  [*] Renamed the primary key on columns (");
                                render_primary_key_column_names(tables.previous, &mut out);
                                out.push_str(")\n");
                            }
                            TableChange::AddPrimaryKey => {
                                out.push_str("  [+] Added primary key on columns (");
                                render_primary_key_column_names(tables.next, &mut out);
                                out.push_str(")\n");
                                out.push_str(")\n");
                            }
                        }
                    }
                }
                SqlMigrationStep::DropTable { table_id } => {
                    out.push_str("  - ");
                    out.push_str(self.schemas().previous.walk(*table_id).name());
                    out.push('\n');
                }
                SqlMigrationStep::DropEnum(enum_id) => {
                    out.push_str("  - ");
                    out.push_str(self.schemas().previous.walk(*enum_id).name());
                    out.push('\n');
                }
                SqlMigrationStep::CreateTable { table_id } => {
                    out.push_str("  - ");
                    out.push_str(self.schemas().next.walk(*table_id).name());
                    out.push('\n');
                }
                SqlMigrationStep::RedefineTables(_) => {}
                SqlMigrationStep::RenameForeignKey { foreign_key_id } => {
                    let fks = self.schemas().walk(*foreign_key_id);
                    out.push_str("  [*] Renamed the foreign key \"");
                    out.push_str(fks.previous.constraint_name().unwrap());
                    out.push_str("\" to \"");
                    out.push_str(fks.next.constraint_name().unwrap());
                    out.push_str("\"\n");
                }
                SqlMigrationStep::CreateIndex {
                    table_id: _,
                    index_id,
                    from_drop_and_recreate: _,
                } => {
                    let index = self.schemas().next.walk(*index_id);

                    out.push_str("  [+] Added ");

                    if index.is_unique() {
                        out.push_str("unique ");
                    }

                    out.push_str("index on columns (");
                    out.push_str(&index.column_names().join(", "));
                    out.push_str(")\n");
                }
                SqlMigrationStep::AddForeignKey { foreign_key_id } => {
                    let foreign_key = self.schemas().next.walk(*foreign_key_id);
                    out.push_str("  [+] Added foreign key on columns (");
                    out.push_str(&foreign_key.constrained_columns().map(|c| c.name()).join(", "));
                    out.push_str(")\n")
                }
                SqlMigrationStep::RenameIndex { index } => {
                    let index = self.schemas().walk(*index);

                    out.push_str("  [*] Renamed index `");
                    out.push_str(index.previous.name());
                    out.push_str("` to `");
                    out.push_str(index.next.name());
                    out.push_str("`\n");
                }
                SqlMigrationStep::RedefineIndex { index } => {
                    let index = self.schemas().walk(*index);

                    out.push_str("  [*] Redefined index `");
                    out.push_str(index.previous.name());
                    out.push_str("`\n");
                }
                SqlMigrationStep::CreateExtension(create_extension) => {
                    let ext: &PostgresSchemaExt = self.schemas().next.downcast_connector_data();
                    out.push_str("  - ");
                    out.push_str(&ext.get_extension(create_extension.id).name);
                    out.push('\n');
                }
                SqlMigrationStep::AlterExtension(_) => {}
                SqlMigrationStep::DropExtension(_) => {}
            }
        }

        out
    }
}

fn render_column_changes(columns: MigrationPair<TableColumnWalker<'_>>, changes: &ColumnChanges, sink: &mut String) {
    let readable_changes = changes
        .iter()
        .map(|change| match change {
            ColumnChange::Arity => format!(
                "changed from {:?} to {:?}",
                columns.previous.arity(),
                columns.next.arity()
            ),
            ColumnChange::Default => format!(
                "default changed from `{:?}` to `{:?}`",
                columns.previous.default().map(|d| d.kind()),
                columns.next.default().map(|d| d.kind())
            ),
            ColumnChange::TypeChanged => "type changed".to_owned(),
            ColumnChange::Autoincrement => {
                if columns.previous.is_autoincrement() {
                    "column is no longer autoincrementing".to_owned()
                } else {
                    "column became autoincrementing".to_owned()
                }
            }
        })
        .join(", ");

    sink.push('(');
    sink.push_str(&readable_changes);
    sink.push(')');
}

// The order of the variants matters for sorting. The steps are sorted _first_
// by variant, then by the contents. Since the contents are mostly indexes in a
// SqlSchema struct, the natural ordering of the indexes matches well with what
// you would intuitively expect.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SqlMigrationStep {
    CreateSchema(sql_schema_describer::NamespaceId),
    DropExtension(DropExtension),
    CreateExtension(CreateExtension),
    AlterExtension(AlterExtension),
    AlterSequence(MigrationPair<u32>, SequenceChanges),
    DropView(DropView),
    DropUserDefinedType(DropUserDefinedType),
    CreateEnum(sql_schema_describer::EnumId),
    AlterEnum(AlterEnum),
    DropForeignKey {
        foreign_key_id: ForeignKeyId,
    },
    DropIndex {
        index_id: IndexId,
    },
    AlterTable(AlterTable),
    AlterPrimaryKey(MigrationPair<TableId>),
    // Order matters: we must drop tables before we create indexes,
    // because on Postgres and SQLite, we may create indexes whose names
    // clash with the names of indexes on the dropped tables.
    DropTable {
        table_id: TableId,
    },
    // Order matters:
    // - We must drop enums before we create tables, because the new tables
    //   might be named the same as the dropped enum, and that conflicts on
    //   postgres.
    // - We must drop enums after we drop tables, or dropping the enum will
    //   fail on postgres because objects (=tables) still depend on them.
    DropEnum(sql_schema_describer::EnumId),
    CreateTable {
        table_id: TableId,
    },
    RedefineTables(Vec<RedefineTable>),
    // Order matters: we must create indexes after ALTER TABLEs because the indexes can be
    // on fields that are dropped/created there.
    CreateIndex {
        table_id: (Option<TableId>, TableId),
        index_id: IndexId,
        from_drop_and_recreate: bool,
    },
    RenameForeignKey {
        foreign_key_id: MigrationPair<ForeignKeyId>,
    },
    // Order matters: this needs to come after create_indexes, because the foreign keys can depend on unique
    // indexes created there.
    AddForeignKey {
        foreign_key_id: ForeignKeyId,
    },
    RenameIndex {
        index: MigrationPair<IndexId>,
    },
    RedefineIndex {
        index: MigrationPair<IndexId>,
    },
}

impl SqlMigrationStep {
    pub(crate) fn description(&self) -> &'static str {
        match self {
            SqlMigrationStep::AddForeignKey { .. } => "AddForeignKey",
            SqlMigrationStep::AlterEnum(_) => "AlterEnum",
            SqlMigrationStep::AlterPrimaryKey(_) => "AlterPrimaryKey",
            SqlMigrationStep::AlterSequence(_, _) => "AlterSequence",
            SqlMigrationStep::AlterTable(_) => "AlterTable",
            SqlMigrationStep::CreateEnum(_) => "CreateEnum",
            SqlMigrationStep::CreateIndex { .. } => "CreateIndex",
            SqlMigrationStep::CreateSchema { .. } => "CreateSchema",
            SqlMigrationStep::CreateTable { .. } => "CreateTable",
            SqlMigrationStep::DropEnum(_) => "DropEnum",
            SqlMigrationStep::DropForeignKey { .. } => "DropForeignKey",
            SqlMigrationStep::DropIndex { .. } => "DropIndex",
            SqlMigrationStep::DropTable { .. } => "DropTable",
            SqlMigrationStep::DropUserDefinedType(_) => "DropUserDefinedType",
            SqlMigrationStep::DropView(_) => "DropView",
            SqlMigrationStep::RedefineIndex { .. } => "RedefineIndex",
            SqlMigrationStep::RedefineTables { .. } => "RedefineTables",
            SqlMigrationStep::RenameForeignKey { .. } => "RenameForeignKey",
            SqlMigrationStep::RenameIndex { .. } => "RenameIndex",
            SqlMigrationStep::CreateExtension(_) => "CreateExtension",
            SqlMigrationStep::AlterExtension(_) => "AlterExtension",
            SqlMigrationStep::DropExtension(_) => "DropExtension",
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AlterExtension {
    pub ids: MigrationPair<postgres::ExtensionId>,
    pub changes: Vec<ExtensionChange>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CreateExtension {
    pub id: postgres::ExtensionId,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DropExtension {
    pub id: postgres::ExtensionId,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ExtensionChange {
    AlterVersion,
    AlterSchema,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AlterTable {
    pub table_ids: MigrationPair<TableId>,
    pub changes: Vec<TableChange>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TableChange {
    AddColumn {
        column_id: TableColumnId,
        has_virtual_default: bool,
    },
    AlterColumn(AlterColumn),
    DropColumn {
        column_id: TableColumnId,
    },
    DropAndRecreateColumn {
        column_id: MigrationPair<TableColumnId>,
        /// The change mask for the column.
        changes: ColumnChanges,
    },
    DropPrimaryKey,
    AddPrimaryKey,
    RenamePrimaryKey,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DropView {
    pub view_id: ViewId,
}

impl DropView {
    pub fn new(view_id: ViewId) -> Self {
        Self { view_id }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DropUserDefinedType {
    pub udt_id: UdtId,
}

impl DropUserDefinedType {
    pub(crate) fn new(udt_id: UdtId) -> Self {
        Self { udt_id }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AlterColumn {
    pub column_id: MigrationPair<TableColumnId>,
    pub changes: ColumnChanges,
    pub type_change: Option<ColumnTypeChange>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ColumnTypeChange {
    RiskyCast,
    SafeCast,
    NotCastable,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AlterEnum {
    pub id: MigrationPair<EnumId>,
    pub created_variants: Vec<String>,
    pub dropped_variants: Vec<String>,
    /// This should be intepreted as prev_colidx, Option<next_colidx>) The second item in the tuple
    /// is `Some` _only_ when the next column has the same enum as a default, such that the default
    /// would need to be reinstalled after the drop.
    #[allow(clippy::type_complexity)]
    pub previous_usages_as_default: Vec<(TableColumnId, Option<TableColumnId>)>,
}

impl AlterEnum {
    pub(crate) fn is_empty(&self) -> bool {
        self.created_variants.is_empty() && self.dropped_variants.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RedefineTable {
    pub added_columns: Vec<TableColumnId>,
    pub added_columns_with_virtual_defaults: Vec<TableColumnId>,
    pub dropped_columns: Vec<TableColumnId>,
    pub dropped_primary_key: bool,
    pub column_pairs: Vec<(MigrationPair<TableColumnId>, ColumnChanges, Option<ColumnTypeChange>)>,
    pub table_ids: MigrationPair<TableId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SequenceChanges(pub(crate) BitFlags<SequenceChange>);

impl PartialOrd for SequenceChanges {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SequenceChanges {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.bits().cmp(&other.0.bits())
    }
}

#[enumflags2::bitflags]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum SequenceChange {
    MinValue = 1,
    MaxValue = 1 << 1,
    Start = 1 << 2,
    Cache = 1 << 3,
    Increment = 1 << 4,
}

fn render_primary_key_column_names(table: TableWalker<'_>, out: &mut String) {
    let cols = table
        .primary_key_columns()
        .into_iter()
        .flatten()
        .map(|c| c.name())
        .join(", ");
    out.push_str(&cols);
}
