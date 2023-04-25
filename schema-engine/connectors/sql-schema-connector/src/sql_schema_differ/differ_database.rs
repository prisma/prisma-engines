use super::{column, enums::EnumDiffer, table::TableDiffer};
use crate::{flavour::SqlFlavour, migration_pair::MigrationPair, SqlDatabaseSchema};
#[cfg(feature = "postgresql")]
use sql_schema_describer::postgres::{ExtensionId, ExtensionWalker, PostgresSchemaExt};
use sql_schema_describer::{
    walkers::{EnumWalker, TableColumnWalker, TableWalker},
    NamespaceId, NamespaceWalker, TableColumnId, TableId,
};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Bound,
};

type Table<'a> = (Option<Cow<'a, str>>, Cow<'a, str>);

pub(crate) struct DifferDatabase<'a> {
    pub(super) flavour: &'a dyn SqlFlavour,
    /// The schemas being diffed
    pub(crate) schemas: MigrationPair<&'a SqlDatabaseSchema>,
    /// Namespace name -> namespace indexes.
    namespaces: HashMap<Cow<'a, str>, MigrationPair<Option<NamespaceId>>>,
    /// Table name -> table indexes.
    tables: HashMap<Table<'a>, MigrationPair<Option<TableId>>>,
    /// (table_idxs, column_name) -> column_idxs. BTreeMap because we want range
    /// queries (-> all the columns in a table).
    columns: BTreeMap<(MigrationPair<TableId>, &'a str), MigrationPair<Option<TableColumnId>>>,
    /// (table_idx, column_idx) -> ColumnChanges
    column_changes: HashMap<MigrationPair<TableColumnId>, column::ColumnChanges>,
    /// Postgres extension name -> extension indexes.
    #[cfg(feature = "postgresql")]
    pub(super) extensions: HashMap<&'a str, MigrationPair<Option<ExtensionId>>>,
    /// Tables that will need to be completely redefined (dropped and recreated) for the migration
    /// to succeed. It needs to be crate public because it is set from the flavour.
    pub(crate) tables_to_redefine: BTreeSet<MigrationPair<TableId>>,
}

impl<'a> DifferDatabase<'a> {
    pub(crate) fn new(schemas: MigrationPair<&'a SqlDatabaseSchema>, flavour: &'a dyn SqlFlavour) -> Self {
        let namespace_count_lb = std::cmp::max(
            schemas.previous.describer_schema.namespaces_count(),
            schemas.next.describer_schema.namespaces_count(),
        );
        let table_count_lb = std::cmp::max(
            schemas.previous.describer_schema.tables_count(),
            schemas.next.describer_schema.tables_count(),
        );

        let mut db = DifferDatabase {
            flavour,
            schemas,
            namespaces: HashMap::with_capacity(namespace_count_lb),
            tables: HashMap::with_capacity(table_count_lb),
            columns: BTreeMap::new(),
            column_changes: Default::default(),
            #[cfg(feature = "postgresql")]
            extensions: Default::default(),
            tables_to_redefine: Default::default(),
        };

        let mut columns_cache = HashMap::new();
        let table_is_ignored = |table_name: &str| {
            table_name == crate::MIGRATIONS_TABLE_NAME || flavour.table_should_be_ignored(table_name)
        };

        // First insert all namespaces from the previous schema.
        for namespace in schemas.previous.describer_schema.walk_namespaces() {
            let namespace_name = if flavour.lower_cases_table_names() {
                namespace.name().to_ascii_lowercase().into()
            } else {
                Cow::Borrowed(namespace.name())
            };
            db.namespaces
                .insert(namespace_name, MigrationPair::new(Some(namespace.id), None));
        }

        // Then insert all namespaces from the next schema.
        for namespace in schemas.next.describer_schema.walk_namespaces() {
            let namespace_name = if flavour.lower_cases_table_names() {
                namespace.name().to_ascii_lowercase().into()
            } else {
                Cow::Borrowed(namespace.name())
            };
            let entry = db.namespaces.entry(namespace_name).or_default();
            entry.next = Some(namespace.id);
        }

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
            db.tables.insert(
                (table.namespace().map(Cow::Borrowed), table_name),
                MigrationPair::new(Some(table.id), None),
            );
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
            let entry = db
                .tables
                .entry((table.namespace().map(Cow::Borrowed), table_name))
                .or_default();
            entry.next = Some(table.id);

            // Deal with tables that are both in the previous and the next
            // schema: we are going to look at heir columns.
            if let Some(table_pair) = entry.transpose() {
                let tables = schemas.walk(table_pair);

                columns_cache.clear();

                // Same as for tables, walk the previous columns first.
                for column in tables.previous.columns() {
                    columns_cache.insert(column.name(), MigrationPair::new(Some(column.id), None));
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
        flavour.define_extensions(&mut db);

        db
    }

    pub(crate) fn all_column_pairs(&self) -> impl Iterator<Item = MigrationPair<TableColumnId>> + '_ {
        self.columns.iter().filter_map(|(_, cols)| cols.transpose())
    }

    pub(crate) fn column_pairs(
        &self,
        table: MigrationPair<TableId>,
    ) -> impl Iterator<Item = MigrationPair<TableColumnId>> + '_ {
        self.range_columns(table).filter_map(|(_k, v)| v.transpose())
    }

    pub(crate) fn column_changes(&self, column: MigrationPair<TableColumnId>) -> column::ColumnChanges {
        self.column_changes[&column]
    }

    pub(crate) fn column_changes_for_walkers(
        &self,
        walkers: MigrationPair<TableColumnWalker<'_>>,
    ) -> column::ColumnChanges {
        self.column_changes(walkers.map(|c| c.id))
    }

    pub(crate) fn created_columns(&self, table: MigrationPair<TableId>) -> impl Iterator<Item = TableColumnId> + '_ {
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

    pub(crate) fn created_namespaces(&self) -> impl Iterator<Item = NamespaceWalker<'_>> + '_ {
        self.namespaces
            .values()
            .filter(|p| p.previous.is_none())
            .filter_map(|p| p.next)
            .map(move |namespace_id| self.schemas.next.walk(namespace_id))
    }

    pub(crate) fn dropped_columns(&self, table: MigrationPair<TableId>) -> impl Iterator<Item = TableColumnId> + '_ {
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
        table: MigrationPair<TableId>,
    ) -> impl Iterator<
        Item = (
            &(MigrationPair<TableId>, &'a str),
            &MigrationPair<Option<TableColumnId>>,
        ),
    > {
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

    pub(crate) fn table_is_redefined(&self, namespace: Option<Cow<'_, str>>, table_name: Cow<'_, str>) -> bool {
        self.tables
            .get(&(namespace, table_name))
            .and_then(|pair| pair.transpose())
            .map(|ids| self.tables_to_redefine.contains(&ids))
            .unwrap_or(false)
    }

    pub(crate) fn enum_pairs(&self) -> impl Iterator<Item = EnumDiffer<'_>> {
        self.previous_enums().filter_map(move |previous| {
            self.next_enums()
                .find(|next| enums_match(&previous, next))
                .map(|next| EnumDiffer {
                    enums: MigrationPair::new(previous, next),
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

    /// Extensions not present in the previous schema.
    #[cfg(feature = "postgresql")]
    pub(crate) fn created_extensions(&self) -> impl Iterator<Item = ExtensionId> + '_ {
        self.extensions
            .values()
            .filter(|pair| pair.previous.is_none())
            .filter_map(|pair| pair.next)
    }

    /// Non-relocatable extensions present in both schemas with changed values.
    #[cfg(feature = "postgresql")]
    pub(crate) fn non_relocatable_extension_pairs<'db>(
        &'db self,
    ) -> impl Iterator<Item = MigrationPair<ExtensionWalker<'a>>> + 'db {
        self.previous_extensions().filter_map(move |previous| {
            self.next_extensions()
                .find(|next| {
                    previous.name() == next.name()
                        && !extensions_match(previous, *next)
                        && (!previous.relocatable() && !next.relocatable())
                })
                .map(|next| MigrationPair::new(previous, next))
        })
    }

    /// Relocatable extensions present in both schemas with changed values.
    #[cfg(feature = "postgresql")]
    pub(crate) fn relocatable_extension_pairs<'db>(
        &'db self,
    ) -> impl Iterator<Item = MigrationPair<ExtensionWalker<'a>>> + 'db {
        self.previous_extensions().filter_map(move |previous| {
            self.next_extensions()
                .find(|next| {
                    previous.name() == next.name()
                        && !extensions_match(previous, *next)
                        && (previous.relocatable() || next.relocatable())
                })
                .map(|next| MigrationPair::new(previous, next))
        })
    }

    fn previous_enums(&self) -> impl Iterator<Item = EnumWalker<'a>> {
        self.schemas.previous.describer_schema.enum_walkers()
    }

    fn next_enums(&self) -> impl Iterator<Item = EnumWalker<'a>> {
        self.schemas.next.describer_schema.enum_walkers()
    }

    #[cfg(feature = "postgresql")]
    fn previous_extensions(&self) -> impl Iterator<Item = ExtensionWalker<'a>> {
        let conn_data: &PostgresSchemaExt = self.schemas.previous.describer_schema.downcast_connector_data();
        conn_data.extension_walkers()
    }

    #[cfg(feature = "postgresql")]
    fn next_extensions(&self) -> impl Iterator<Item = ExtensionWalker<'a>> {
        let conn_data: &PostgresSchemaExt = self.schemas.next.describer_schema.downcast_connector_data();
        conn_data.extension_walkers()
    }
}

#[cfg(feature = "postgresql")]
pub(crate) fn extensions_match(previous: ExtensionWalker<'_>, next: ExtensionWalker<'_>) -> bool {
    let names_match = previous.name() == next.name();

    let versions_match =
        previous.version() == next.version() || previous.version().is_empty() || next.version().is_empty();

    let schemas_match = previous.schema() == next.schema() || previous.schema().is_empty() || next.schema().is_empty();

    names_match && versions_match && schemas_match
}

fn enums_match(previous: &EnumWalker<'_>, next: &EnumWalker<'_>) -> bool {
    previous.name() == next.name() && previous.namespace() == next.namespace()
}
