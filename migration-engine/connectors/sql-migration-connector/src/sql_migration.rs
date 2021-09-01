use crate::{
    pair::Pair,
    sql_renderer::IteratorJoin,
    sql_schema_differ::{ColumnChange, ColumnChanges},
};
use sql_schema_describer::{
    walkers::{ColumnWalker, SqlSchemaExt},
    ColumnId, SqlSchema, TableId,
};
use std::{collections::BTreeSet, fmt::Write as _};

/// The database migration type for SqlMigrationConnector.
#[derive(Debug)]
pub struct SqlMigration {
    pub(crate) before: SqlSchema,
    pub(crate) after: SqlSchema,
    /// (table_id, column_id) for columns with a prisma-level default
    /// (cuid() or uuid()) in the `after` schema that aren't present in the
    /// `before` schema.
    pub(crate) added_columns_with_virtual_defaults: Vec<(TableId, ColumnId)>,
    pub(crate) steps: Vec<SqlMigrationStep>,
}

impl SqlMigration {
    pub(crate) fn schemas(&self) -> Pair<&SqlSchema> {
        Pair::new(&self.before, &self.after)
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
        // The order of the variants matters
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
        #[repr(u8)]
        enum DriftType {
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
                SqlMigrationStep::DropView(drop_view) => {
                    drift_items.insert((
                        DriftType::RemovedView,
                        self.schemas().previous().view_walker_at(drop_view.view_index).name(),
                        idx,
                    ));
                }
                SqlMigrationStep::DropUserDefinedType(drop_udt) => {
                    drift_items.insert((
                        DriftType::RemovedUdt,
                        self.schemas().previous().udt_walker_at(drop_udt.udt_index).name(),
                        idx,
                    ));
                }
                SqlMigrationStep::CreateEnum { .. } => {
                    drift_items.insert((DriftType::AddedEnum, "", idx));
                }
                SqlMigrationStep::AlterEnum(alter_enum) => {
                    drift_items.insert((
                        DriftType::ChangedEnum,
                        self.schemas().enums(&alter_enum.index).previous().name(),
                        idx,
                    ));
                }
                SqlMigrationStep::DropForeignKey { table_id, .. } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().previous().table_walker_at(*table_id).name(),
                        idx,
                    ));
                }
                SqlMigrationStep::DropIndex { table_id, .. } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().previous().table_walker_at(*table_id).name(),
                        idx,
                    ));
                }
                SqlMigrationStep::AlterTable(alter_table) => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().tables(&alter_table.table_ids).previous().name(),
                        idx,
                    ));
                }
                SqlMigrationStep::DropTable { .. } => {
                    drift_items.insert((DriftType::RemovedTable, "", idx));
                }
                SqlMigrationStep::DropEnum { .. } => {
                    drift_items.insert((DriftType::RemovedEnum, "", idx));
                }
                SqlMigrationStep::CreateTable { .. } => {
                    drift_items.insert((DriftType::AddedTable, "", idx));
                }
                SqlMigrationStep::RedefineTables(redefines) => {
                    for redefine in redefines {
                        drift_items.insert((
                            DriftType::RedefinedTable,
                            self.schemas().tables(&redefine.table_ids).previous().name(),
                            idx,
                        ));
                    }
                }
                SqlMigrationStep::RenameForeignKey {
                    table_id,
                    foreign_key_id: _,
                } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().tables(table_id).next().name(),
                        idx,
                    ));
                }
                SqlMigrationStep::CreateIndex {
                    table_id: (_, table_id),
                    ..
                } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().next().table_walker_at(*table_id).name(),
                        idx,
                    ));
                }
                SqlMigrationStep::AddForeignKey { table_id, .. } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().next().table_walker_at(*table_id).name(),
                        idx,
                    ));
                }
                SqlMigrationStep::RenameIndex { table, .. } | SqlMigrationStep::RedefineIndex { table, .. } => {
                    drift_items.insert((
                        DriftType::ChangedTable,
                        self.schemas().tables(table).previous().name(),
                        idx,
                    ));
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
                }
            }

            render_state = (*new_state, *item_name);

            match &self.steps[*step_idx as usize] {
                SqlMigrationStep::DropView(_) => {}
                SqlMigrationStep::DropUserDefinedType(_) => {}
                SqlMigrationStep::CreateEnum { enum_index } => {
                    out.push_str("  - ");
                    out.push_str(self.schemas().next().enum_walker_at(*enum_index).name());
                    out.push('\n');
                }
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
                SqlMigrationStep::DropForeignKey {
                    foreign_key_index,
                    table_id,
                } => {
                    let fk = self
                        .schemas()
                        .previous()
                        .table_walker_at(*table_id)
                        .foreign_key_at(*foreign_key_index);

                    out.push_str("  [-] Removed foreign key on columns (");
                    out.push_str(&fk.constrained_column_names().join(", "));
                    out.push_str(")\n")
                }
                SqlMigrationStep::DropIndex { table_id, index_index } => {
                    let index = self
                        .schemas()
                        .previous()
                        .table_walker_at(*table_id)
                        .index_at(*index_index);

                    out.push_str("  [-] Removed ");

                    if index.index_type().is_unique() {
                        out.push_str("unique ");
                    }

                    out.push_str("index on columns (");
                    out.push_str(&index.column_names().join(", "));
                    out.push_str(")\n");
                }
                SqlMigrationStep::AlterTable(alter_table) => {
                    let tables = self.schemas().tables(&alter_table.table_ids);

                    for change in &alter_table.changes {
                        match change {
                            TableChange::AddColumn { column_id } => {
                                out.push_str("  [+] Added column `");
                                out.push_str(tables.next().column_at(*column_id).name());
                                out.push_str("`\n");
                            }
                            TableChange::AlterColumn(alter_column) => {
                                out.push_str("  [*] Altered column `");
                                write!(
                                    out,
                                    "{}` ",
                                    tables.next().column_at(*alter_column.column_id.next()).name(),
                                )
                                .unwrap();
                                render_column_changes(
                                    tables.columns(&alter_column.column_id),
                                    &alter_column.changes,
                                    &mut out,
                                );
                                out.push('\n');
                            }
                            TableChange::DropColumn { column_id } => {
                                out.push_str("  [-] Removed column `");
                                out.push_str(tables.previous().column_at(*column_id).name());
                                out.push_str("`\n");
                            }
                            TableChange::DropAndRecreateColumn { column_id, changes } => {
                                out.push_str("  [*] Column `");
                                write!(
                                    out,
                                    "{}` would be dropped and recreated",
                                    tables.next().column_at(*column_id.next()).name(),
                                )
                                .unwrap();
                                render_column_changes(tables.columns(column_id), changes, &mut out);
                                out.push('\n');
                            }
                            TableChange::DropPrimaryKey => {
                                out.push_str("  [-] Dropped the primary key on columns (");
                                out.push_str(&tables.previous().primary_key_column_names().unwrap().join(", "));
                                out.push_str(")\n");
                            }
                            TableChange::RenamePrimaryKey => {
                                out.push_str("  [*] Renamed the primary key on columns (");
                                out.push_str(&tables.previous().primary_key_column_names().unwrap().join(", "));
                                out.push_str(")\n");
                            }
                            TableChange::AddPrimaryKey => {
                                out.push_str("  [+] Added primary key on columns (");
                                out.push_str(&tables.next().primary_key_column_names().unwrap().join(", "));
                                out.push_str(")\n");
                            }
                        }
                    }
                }
                SqlMigrationStep::DropTable { table_id } => {
                    out.push_str("  - ");
                    out.push_str(self.schemas().previous().table_walker_at(*table_id).name());
                    out.push('\n');
                }
                SqlMigrationStep::DropEnum { enum_index } => {
                    out.push_str("  - ");
                    out.push_str(self.schemas().previous().enum_walker_at(*enum_index).name());
                    out.push('\n');
                }
                SqlMigrationStep::CreateTable { table_id } => {
                    out.push_str("  - ");
                    out.push_str(self.schemas().next().table_walker_at(*table_id).name());
                    out.push('\n');
                }
                SqlMigrationStep::RedefineTables(_) => {}
                SqlMigrationStep::RenameForeignKey {
                    table_id,
                    foreign_key_id,
                } => {
                    let fks = self.schemas().tables(table_id).foreign_keys(foreign_key_id);
                    out.push_str("  [*] Renamed the foreign key \"");
                    out.push_str(fks.previous().constraint_name().unwrap());
                    out.push_str("\" to \"");
                    out.push_str(fks.next().constraint_name().unwrap());
                    out.push_str("\"\n");
                }
                SqlMigrationStep::CreateIndex {
                    table_id: (_, table_id),
                    index_index,
                } => {
                    let index = self.schemas().next().table_walker_at(*table_id).index_at(*index_index);

                    out.push_str("  [+] Added ");

                    if index.index_type().is_unique() {
                        out.push_str("unique ");
                    }

                    out.push_str("index on columns (");
                    out.push_str(&index.column_names().join(", "));
                    out.push_str(")\n");
                }
                SqlMigrationStep::AddForeignKey {
                    table_id,
                    foreign_key_index,
                } => {
                    let foreign_key = self
                        .schemas()
                        .next()
                        .table_walker_at(*table_id)
                        .foreign_key_at(*foreign_key_index);

                    out.push_str("  [+] Added foreign key on columns (");
                    out.push_str(&foreign_key.constrained_column_names().join(", "));
                    out.push_str(")\n")
                }
                SqlMigrationStep::RenameIndex { table, index } => {
                    let index = self.schemas().tables(table).indexes(index);

                    out.push_str("  [*] Renamed index `");
                    out.push_str(index.previous().name());
                    out.push_str("` to `");
                    out.push_str(index.next().name());
                    out.push_str("`\n");
                }
                SqlMigrationStep::RedefineIndex { table, index } => {
                    let index = self.schemas().tables(table).indexes(index);

                    out.push_str("  [*] Redefined index `");
                    out.push_str(index.previous().name());
                    out.push_str("`\n");
                }
            }
        }

        out
    }
}

fn render_column_changes(columns: Pair<ColumnWalker<'_>>, changes: &ColumnChanges, sink: &mut String) {
    let readable_changes = changes
        .iter()
        .map(|change| match change {
            ColumnChange::Renaming => "column was renamed".to_owned(),
            ColumnChange::Arity => format!(
                "changed from {:?} to {:?}",
                columns.previous().arity(),
                columns.next().arity()
            ),
            ColumnChange::Default => format!(
                "default changed from `{:?}` to `{:?}`",
                columns.previous().default().map(|d| d.kind()),
                columns.next().default().map(|d| d.kind())
            ),
            ColumnChange::TypeChanged => "type changed".to_owned(),
            ColumnChange::Sequence => "sequence changed".to_owned(),
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
    DropView(DropView),
    DropUserDefinedType(DropUserDefinedType),
    CreateEnum {
        enum_index: usize,
    },
    AlterEnum(AlterEnum),
    DropForeignKey {
        table_id: TableId,
        foreign_key_index: usize,
    },
    DropIndex {
        table_id: TableId,
        index_index: usize,
    },
    AlterTable(AlterTable),
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
    DropEnum {
        enum_index: usize,
    },
    CreateTable {
        table_id: TableId,
    },
    RedefineTables(Vec<RedefineTable>),
    // Order matters: we must create indexes after ALTER TABLEs because the indexes can be
    // on fields that are dropped/created there.
    CreateIndex {
        table_id: (Option<TableId>, TableId),
        index_index: usize,
    },
    RenameForeignKey {
        table_id: Pair<TableId>,
        foreign_key_id: Pair<usize>,
    },
    // Order matters: this needs to come after create_indexes, because the foreign keys can depend on unique
    // indexes created there.
    AddForeignKey {
        /// The index of the table in the next schema.
        table_id: TableId,
        /// The index of the foreign key in the table.
        foreign_key_index: usize,
    },
    RenameIndex {
        table: Pair<TableId>,
        index: Pair<usize>,
    },
    RedefineIndex {
        table: Pair<TableId>,
        index: Pair<usize>,
    },
}

impl SqlMigrationStep {
    pub(crate) fn as_alter_table(&self) -> Option<&AlterTable> {
        match self {
            SqlMigrationStep::AlterTable(alter_table) => Some(alter_table),
            _ => None,
        }
    }

    pub(crate) fn as_redefine_tables(&self) -> Option<&[RedefineTable]> {
        match self {
            SqlMigrationStep::RedefineTables(redefines) => Some(redefines),
            _ => None,
        }
    }

    pub(crate) fn description(&self) -> &'static str {
        match self {
            SqlMigrationStep::AddForeignKey { .. } => "AddForeignKey",
            SqlMigrationStep::AlterEnum(_) => "AlterEnum",
            SqlMigrationStep::AlterTable(_) => "AlterTable",
            SqlMigrationStep::CreateEnum { .. } => "CreateEnum",
            SqlMigrationStep::CreateIndex { .. } => "CreateIndex",
            SqlMigrationStep::CreateTable { .. } => "CreateTable",
            SqlMigrationStep::DropEnum { .. } => "DropEnum",
            SqlMigrationStep::DropForeignKey { .. } => "DropForeignKey",
            SqlMigrationStep::DropIndex { .. } => "DropIndex",
            SqlMigrationStep::DropTable { .. } => "DropTable",
            SqlMigrationStep::DropUserDefinedType(_) => "DropUserDefinedType",
            SqlMigrationStep::DropView(_) => "DropView",
            SqlMigrationStep::RedefineIndex { .. } => "RedefineIndex",
            SqlMigrationStep::RedefineTables { .. } => "RedefineTables",
            SqlMigrationStep::RenameForeignKey { .. } => "RenameForeignKey",
            SqlMigrationStep::RenameIndex { .. } => "RenameIndex",
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AlterTable {
    pub table_ids: Pair<TableId>,
    pub changes: Vec<TableChange>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TableChange {
    AddColumn {
        column_id: ColumnId,
    },
    AlterColumn(AlterColumn),
    DropColumn {
        column_id: ColumnId,
    },
    DropAndRecreateColumn {
        column_id: Pair<ColumnId>,
        /// The change mask for the column.
        changes: ColumnChanges,
    },
    DropPrimaryKey,
    AddPrimaryKey,
    RenamePrimaryKey,
}

impl TableChange {
    pub(crate) fn as_add_column(&self) -> Option<ColumnId> {
        match self {
            TableChange::AddColumn { column_id } => Some(*column_id),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DropView {
    pub view_index: usize,
}

impl DropView {
    pub fn new(view_index: usize) -> Self {
        Self { view_index }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DropUserDefinedType {
    pub udt_index: usize,
}

impl DropUserDefinedType {
    pub(crate) fn new(udt_index: usize) -> Self {
        Self { udt_index }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DropColumn {
    pub index: usize,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AlterColumn {
    pub column_id: Pair<ColumnId>,
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
    pub index: Pair<usize>,
    pub created_variants: Vec<String>,
    pub dropped_variants: Vec<String>,
    /// This should be intepreted as ((prev_tblidx, prev_colidx),
    /// Option<(next_tblidx, next_colidx)>) The second item in the tuple is
    /// `Some` _only_ when the next column has the same enum as a default, such
    /// that the default would need to be reinstalled after the drop.
    #[allow(clippy::type_complexity)]
    pub previous_usages_as_default: Vec<((TableId, ColumnId), Option<(TableId, ColumnId)>)>,
}

impl AlterEnum {
    pub(crate) fn is_empty(&self) -> bool {
        self.created_variants.is_empty() && self.dropped_variants.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RedefineTable {
    pub added_columns: Vec<ColumnId>,
    pub dropped_columns: Vec<ColumnId>,
    pub dropped_primary_key: bool,
    pub column_pairs: Vec<(Pair<ColumnId>, ColumnChanges, Option<ColumnTypeChange>)>,
    pub table_ids: Pair<TableId>,
}
