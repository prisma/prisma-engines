use crate::{flavour::SqlFlavour, pair::Pair};
use sql_schema_describer::{
    walkers::{ColumnWalker, EnumWalker, ForeignKeyWalker, IndexWalker, SqlSchemaExt, TableWalker},
    ColumnTypeFamily, PrimaryKey, SqlSchema,
};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    ops::Bound,
};

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
struct TableName<'a>(Cow<'a, str>);

impl TableName<'_> {
    fn new_case_sensitive(name: &str) -> TableName<'_> {
        TableName(Cow::Borrowed(name))
    }

    fn new_case_insensitive(name: &str) -> TableName<'_> {
        TableName(Cow::Owned(name.to_ascii_lowercase()))
    }
}

pub(crate) struct DifferDatabase<'a> {
    schemas: Pair<&'a SqlSchema>,
    /// Table name -> table indexes.
    tables: HashMap<TableName<'a>, Pair<Option<usize>>>,
    /// (table_idxs, column_name) -> column_idxs
    columns: BTreeMap<(Pair<usize>, &'a str), Pair<Option<usize>>>,
    flavour: &'a dyn SqlFlavour,
}

impl<'a> DifferDatabase<'a> {
    pub(crate) fn new(schemas: Pair<&'a SqlSchema>, flavour: &'a dyn SqlFlavour) -> Self {
        let table_count_lb = std::cmp::max(schemas.previous().tables.len(), schemas.next().tables.len());
        let mut tables = HashMap::with_capacity(table_count_lb);
        let mut columns = BTreeMap::<(Pair<usize>, &'a str), Pair<Option<usize>>>::new();
        let mut columns_cache = HashMap::new();
        let new_table_name: &dyn Fn(&str) -> TableName<'_> = if flavour.lower_cases_table_names() {
            &TableName::new_case_insensitive
        } else {
            &TableName::new_case_sensitive
        };

        for table in schemas.previous().table_walkers() {
            let table_name = new_table_name(table.name());
            tables.insert(table_name, Pair::new(Some(table.table_index()), None));
        }

        for table in schemas.next().table_walkers() {
            let table_name = new_table_name(table.name());
            let entry = tables.entry(table_name).or_default();
            *entry.next_mut() = Some(table.table_index());

            if let Some(table_pair) = entry.transpose() {
                let tables = schemas.tables(&table_pair);

                columns_cache.clear();

                for column in tables.previous().columns() {
                    columns_cache.insert(column.name(), Pair::new(Some(column.column_index()), None));
                }

                for column in tables.next().columns() {
                    let mut entry = columns_cache.entry(column.name()).or_default();
                    *entry.next_mut() = Some(column.column_index());
                }

                for (column_name, indexes) in &columns_cache {
                    columns.insert((table_pair, column_name), *indexes);
                }
            }
        }

        DifferDatabase {
            tables,
            columns,
            schemas,
            flavour,
        }
    }

    pub(crate) fn created_tables(&self) -> impl Iterator<Item = TableWalker<'a>> + '_ {
        self.tables
            .values()
            .filter(|p| p.previous().is_none())
            .filter_map(|p| *p.next())
            .map(move |idx| self.schemas.next().table_walker_at(idx))
    }

    pub(crate) fn dropped_tables(&self) -> impl Iterator<Item = TableWalker<'a>> + '_ {
        self.tables
            .values()
            .filter(|p| p.next().is_none())
            .filter_map(|p| *p.previous())
            .map(move |idx| self.schemas.previous().table_walker_at(idx))
    }

    /// An iterator over the tables that are present in both schemas.
    pub(crate) fn table_pairs(&self) -> impl Iterator<Item = Pair<TableWalker<'a>>> + '_ {
        self.tables
            .values()
            .filter_map(|p| p.transpose())
            .map(move |idxs| self.schemas.tables(&idxs))
    }

    fn range_columns(
        &self,
        table: Pair<usize>,
    ) -> impl Iterator<Item = (&(Pair<usize>, &'a str), &Pair<Option<usize>>)> {
        self.columns
            .range((Bound::Included(&(table, "")), Bound::Unbounded))
            .take_while(move |((t, _), _)| *t == table)
    }

    pub(crate) fn created_columns(&self, table: Pair<usize>) -> impl Iterator<Item = usize> + '_ {
        self.range_columns(table)
            .filter(|(k, v)| v.previous().is_none())
            .filter_map(|(k, v)| *v.next())
    }

    /// The primary key present in `next` but not `previous`, if applicable.
    pub(crate) fn created_primary_key(&self, table: Pair<usize>) -> Option<&'a PrimaryKey> {
        match self.as_ref().map(|t| t.primary_key()).as_tuple() {
            (None, Some(pk)) => Some(pk),
            (Some(previous_pk), Some(next_pk)) if previous_pk.columns != next_pk.columns => Some(next_pk),
            (Some(previous_pk), Some(next_pk)) => {
                if self.primary_key_column_changed(previous_pk) {
                    Some(next_pk)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn dropped_foreign_keys(&self, table: Pair<usize>) -> impl Iterator<Item = ForeignKeyWalker<'a>> + '_ {
        let tables = self.schemas.tables(&table);

        tables.previous().foreign_keys().filter(move |fk| {
            !tables
                .next()
                .foreign_keys()
                .any(|next_fk| self.foreign_keys_match(Pair::new(fk, next_fk)))
        })
    }

    pub(crate) fn dropped_columns(&self, table: Pair<usize>) -> impl Iterator<Item = usize> + '_ {
        self.range_columns(table)
            .filter(|(k, v)| v.next().is_none())
            .filter_map(|(k, v)| *v.previous())
    }

    fn enums_match(previous: &EnumWalker<'_>, next: &EnumWalker<'_>) -> bool {
        previous.name() == next.name()
    }

    pub(crate) fn foreign_key_pairs(
        &self,
        table: Pair<usize>,
    ) -> impl Iterator<Item = Pair<ForeignKeyWalker<'a>>> + '_ {
        let tables = self.schemas.tables(&table);

        tables.previous().foreign_keys().filter_map(move |fk| {
            tables
                .next()
                .foreign_keys()
                .find(|next_fk| self.foreign_keys_match(fk, next_fk))
                .map(|next_fk| Pair::new(fk, next_fk))
        })
    }

    /// Compare two [ForeignKey](/sql-schema-describer/struct.ForeignKey.html)s and return whether they
    /// should be considered equivalent for schema diffing purposes.
    fn foreign_keys_match(&self, fks: Pair<&ForeignKeyWalker<'_>>) -> bool {
        let references_same_table = flavour.table_names_match(fks.map(|fk| fk.referenced_table().name()));
        let references_same_column_count =
            fks.previous().referenced_columns_count() == fks.next().referenced_columns_count();
        let constrains_same_column_count =
            fks.previous().constrained_columns().count() == fks.next().constrained_columns().count();
        let constrains_same_columns = fks.interleave(|fk| fk.constrained_columns()).all(|fks| {
            let families_match = match fks.map(|fk| fk.column_type_family()).as_tuple() {
                (ColumnTypeFamily::Uuid, ColumnTypeFamily::String) => true,
                (ColumnTypeFamily::String, ColumnTypeFamily::Uuid) => true,
                (x, y) => x == y,
            };

            fks.previous().name() == fks.next().name() && families_match
        });

        // Foreign key references different columns or the same columns in a different order.
        let references_same_columns = fks
            .interleave(|fk| fk.referenced_column_names())
            .all(|pair| pair.previous() == pair.next());

        references_same_table
            && references_same_column_count
            && constrains_same_column_count
            && constrains_same_columns
            && references_same_columns
    }

    /// Compare two SQL indexes and return whether they only differ by name.
    fn indexes_match(&self, first: &IndexWalker<'_>, second: &IndexWalker<'_>) -> bool {
        first.column_names() == second.column_names() && first.index_type() == second.index_type()
    }

    pub(crate) fn walk_column_pairs<'b>(
        &'b self,
        table: &'b Pair<TableWalker<'a>>,
    ) -> impl Iterator<Item = Pair<ColumnWalker<'a>>> + 'b {
        self.range_columns(table.table_indexes())
            .filter_map(|(k, v)| v.transpose())
            .map(move |c| table.columns(&c))
    }
}
