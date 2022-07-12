use super::{column, enums::EnumDiffer, table::TableDiffer};
use crate::{flavour::SqlFlavour, pair::Pair, SqlDatabaseSchema};
use sql_schema_describer::{
    walkers::{ColumnWalker, EnumWalker, TableWalker},
    ColumnId, TableId,
};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Bound,
};

pub(crate) struct DifferDatabase<'a> {
    pub(super) flavour: &'a dyn SqlFlavour,
    /// The schemas being diffed
    schemas: Pair<&'a SqlDatabaseSchema>,
    /// Table name -> table indexes.
    tables: HashMap<Cow<'a, str>, Pair<Option<TableId>>>,
    /// (table_idxs, column_name) -> column_idxs. BTreeMap because we want range
    /// queries (-> all the columns in a table).
    columns: BTreeMap<(Pair<TableId>, &'a str), Pair<Option<ColumnId>>>,
    /// (table_idx, column_idx) -> ColumnChanges
    column_changes: HashMap<Pair<ColumnId>, column::ColumnChanges>,
    /// Tables that will need to be completely redefined (dropped and recreated) for the migration
    /// to succeed. It needs to be crate public because it is set from the flavour.
    pub(crate) tables_to_redefine: BTreeSet<Pair<TableId>>,
}

impl<'a> DifferDatabase<'a> {
    pub(crate) fn new(schemas: Pair<&'a SqlDatabaseSchema>, flavour: &'a dyn SqlFlavour) -> Self {
        let table_count_lb = std::cmp::max(
            schemas.previous.describer_schema.tables_count(),
            schemas.next.describer_schema.tables_count(),
        );
        let mut db = DifferDatabase {
            flavour,
            schemas,
            tables: HashMap::with_capacity(table_count_lb),
            columns: BTreeMap::new(),
            column_changes: Default::default(),
            tables_to_redefine: Default::default(),
        };

        let mut columns_cache = HashMap::new();
        let table_is_ignored =
            |table_name: &str| table_name == "_prisma_migrations" || flavour.table_should_be_ignored(table_name);

        // First insert all tables from the previous schema.
        for table in schemas
            .previous
            .describer_schema
            .table_walkers()
            .filter(|t| !table_is_ignored(t.name()))
        {
            let table_name = if flavour.lower_cases_table_names() {
                table.name().to_ascii_lowercase().into()
            } else {
                Cow::Borrowed(table.name())
            };
            db.tables.insert(table_name, Pair::new(Some(table.id), None));
        }

        // Then insert all tables from the next schema. Since we have all the
        // relevant tables, we can fill in columns at this step.
        for table in schemas
            .next
            .describer_schema
            .table_walkers()
            .filter(|t| !table_is_ignored(t.name()))
        {
            let table_name = if flavour.lower_cases_table_names() {
                table.name().to_ascii_lowercase().into()
            } else {
                Cow::Borrowed(table.name())
            };
            let entry = db.tables.entry(table_name).or_default();
            entry.next = Some(table.id);

            // Deal with tables that are both in the previous and the next
            // schema: we are going to look at heir columns.
            if let Some(table_pair) = entry.transpose() {
                let tables = schemas.walk(table_pair);

                columns_cache.clear();

                // Same as for tables, walk the previous columns first.
                for column in tables.previous.columns() {
                    columns_cache.insert(column.name(), Pair::new(Some(column.id), None));
                }

                for column in tables.next.columns() {
                    let entry = columns_cache.entry(column.name()).or_default();
                    entry.next = Some(column.id);
                }

                // Special treatment for columns that are in both previous and
                // next table: diff the column.
                for (column_name, column_ids) in &columns_cache {
                    db.columns.insert((table_pair, column_name), *column_ids);

                    if let Some(column_ids) = column_ids.transpose() {
                        let column_walkers = schemas.walk(column_ids);
                        let changes = column::all_changes(column_walkers, flavour);
                        db.column_changes.insert(column_ids, changes);
                    }
                }
            }
        }

        flavour.set_tables_to_redefine(&mut db);

        db
    }

    pub(crate) fn all_column_pairs(&self) -> impl Iterator<Item = Pair<ColumnId>> + '_ {
        self.columns.iter().filter_map(|(_, cols)| cols.transpose())
    }

    pub(crate) fn column_pairs(&self, table: Pair<TableId>) -> impl Iterator<Item = Pair<ColumnId>> + '_ {
        self.range_columns(table).filter_map(|(_k, v)| v.transpose())
    }

    pub(crate) fn column_changes(&self, column: Pair<ColumnId>) -> column::ColumnChanges {
        self.column_changes[&column]
    }

    pub(crate) fn column_changes_for_walkers(&self, walkers: Pair<ColumnWalker<'_>>) -> column::ColumnChanges {
        self.column_changes(walkers.map(|c| c.id))
    }

    pub(crate) fn created_columns(&self, table: Pair<TableId>) -> impl Iterator<Item = ColumnId> + '_ {
        self.range_columns(table)
            .filter(|(_k, v)| v.previous.is_none())
            .filter_map(|(_k, v)| v.next)
    }

    pub(crate) fn created_tables(&self) -> impl Iterator<Item = TableWalker<'_>> + '_ {
        self.tables
            .values()
            .filter(|p| p.previous.is_none())
            .filter_map(|p| p.next)
            .map(move |table_id| self.schemas.next.walk(table_id))
    }

    pub(crate) fn dropped_columns(&self, table: Pair<TableId>) -> impl Iterator<Item = ColumnId> + '_ {
        self.range_columns(table)
            .filter(|(_k, v)| v.next.is_none())
            .filter_map(|(_k, v)| v.previous)
    }

    pub(crate) fn dropped_tables(&self) -> impl Iterator<Item = TableWalker<'a>> + '_ {
        self.tables
            .values()
            .filter(|p| p.next.is_none())
            .filter_map(|p| p.previous)
            .map(move |table_id| self.schemas.previous.walk(table_id))
    }

    fn range_columns(
        &self,
        table: Pair<TableId>,
    ) -> impl Iterator<Item = (&(Pair<TableId>, &'a str), &Pair<Option<ColumnId>>)> {
        self.columns
            .range((Bound::Included(&(table, "")), Bound::Unbounded))
            .take_while(move |((t, _), _)| *t == table)
    }

    /// An iterator over the tables that are present in both schemas.
    pub(crate) fn table_pairs<'db>(&'db self) -> impl Iterator<Item = TableDiffer<'a, 'db>> + 'db {
        self.tables
            .values()
            .filter_map(|p| p.transpose())
            .map(move |table_ids| TableDiffer {
                tables: self.schemas.walk(table_ids),
                db: self,
            })
    }

    /// Same as `table_pairs()`, but with the redefined tables filtered out.
    pub(crate) fn non_redefined_table_pairs<'db>(&'db self) -> impl Iterator<Item = TableDiffer<'a, 'db>> + 'db {
        self.table_pairs()
            .filter(move |differ| !self.tables_to_redefine.contains(&differ.table_ids()))
    }

    pub(crate) fn table_is_redefined(&self, table_name: &str) -> bool {
        self.tables
            .get(table_name)
            .and_then(|pair| pair.transpose())
            .map(|ids| self.tables_to_redefine.contains(&ids))
            .unwrap_or(false)
    }

    pub(crate) fn enum_pairs(&self) -> impl Iterator<Item = EnumDiffer<'_>> {
        self.previous_enums().filter_map(move |previous| {
            self.next_enums()
                .find(|next| enums_match(&previous, next))
                .map(|next| EnumDiffer {
                    enums: Pair::new(previous, next),
                })
        })
    }

    pub(crate) fn created_enums<'db>(&'db self) -> impl Iterator<Item = EnumWalker<'a>> + 'db {
        self.next_enums()
            .filter(move |next| !self.previous_enums().any(|previous| enums_match(&previous, next)))
    }

    pub(crate) fn dropped_enums<'db>(&'db self) -> impl Iterator<Item = EnumWalker<'a>> + 'db {
        self.previous_enums()
            .filter(move |previous| !self.next_enums().any(|next| enums_match(previous, &next)))
    }

    fn previous_enums(&self) -> impl Iterator<Item = EnumWalker<'a>> {
        self.schemas.previous.describer_schema.enum_walkers()
    }

    fn next_enums(&self) -> impl Iterator<Item = EnumWalker<'a>> {
        self.schemas.next.describer_schema.enum_walkers()
    }

    pub(crate) fn schemas(&self) -> Pair<&'a SqlDatabaseSchema> {
        self.schemas
    }
}

fn enums_match(previous: &EnumWalker<'_>, next: &EnumWalker<'_>) -> bool {
    previous.name() == next.name()
}
