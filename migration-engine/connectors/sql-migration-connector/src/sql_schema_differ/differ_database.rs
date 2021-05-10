use crate::{flavour::SqlFlavour, pair::Pair};
use sql_schema_describer::{
    walkers::{SqlSchemaExt, TableWalker},
    SqlSchema,
};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    ops::Bound,
};

pub(crate) struct DifferDatabase<'a> {
    schemas: Pair<&'a SqlSchema>,
    /// Table name -> table indexes.
    tables: HashMap<Cow<'a, str>, Pair<Option<usize>>>,
    /// (table_idxs, column_name) -> column_idxs
    columns: BTreeMap<(Pair<usize>, &'a str), Pair<Option<usize>>>,
}

impl<'a> DifferDatabase<'a> {
    pub(crate) fn new(schemas: Pair<&'a SqlSchema>, flavour: &'a dyn SqlFlavour) -> Self {
        let table_count_lb = std::cmp::max(schemas.previous().tables.len(), schemas.next().tables.len());
        let mut tables = HashMap::with_capacity(table_count_lb);
        let mut columns = BTreeMap::<(Pair<usize>, &'a str), Pair<Option<usize>>>::new();
        let mut columns_cache = HashMap::new();
        let table_is_ignored =
            |table_name: &str| table_name == "_prisma_migrations" || flavour.table_should_be_ignored(&table_name);

        for table in schemas
            .previous()
            .table_walkers()
            .filter(|t| !table_is_ignored(t.name()))
        {
            let table_name = if flavour.lower_cases_table_names() {
                table.name().to_ascii_lowercase().into()
            } else {
                Cow::Borrowed(table.name())
            };
            tables.insert(table_name, Pair::new(Some(table.table_index()), None));
        }

        for table in schemas.next().table_walkers().filter(|t| !table_is_ignored(t.name())) {
            let table_name = if flavour.lower_cases_table_names() {
                table.name().to_ascii_lowercase().into()
            } else {
                Cow::Borrowed(table.name())
            };
            let entry = tables.entry(table_name).or_default();
            *entry.next_mut() = Some(table.table_index());

            if let Some(table_pair) = entry.transpose() {
                let tables = schemas.tables(&table_pair);

                columns_cache.clear();

                for column in tables.previous().columns() {
                    columns_cache.insert(column.name(), Pair::new(Some(column.column_index()), None));
                }

                for column in tables.next().columns() {
                    let entry = columns_cache.entry(column.name()).or_default();
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
        }
    }

    pub(crate) fn column_pairs(&self, table: Pair<usize>) -> impl Iterator<Item = Pair<usize>> + '_ {
        self.range_columns(table).filter_map(|(_k, v)| v.transpose())
    }

    pub(crate) fn created_columns(&self, table: Pair<usize>) -> impl Iterator<Item = usize> + '_ {
        self.range_columns(table)
            .filter(|(_k, v)| v.previous().is_none())
            .filter_map(|(_k, v)| *v.next())
    }

    pub(crate) fn created_tables(&self) -> impl Iterator<Item = TableWalker<'a>> + '_ {
        self.tables
            .values()
            .filter(|p| p.previous().is_none())
            .filter_map(|p| *p.next())
            .map(move |idx| self.schemas.next().table_walker_at(idx))
    }

    pub(crate) fn dropped_columns(&self, table: Pair<usize>) -> impl Iterator<Item = usize> + '_ {
        self.range_columns(table)
            .filter(|(_k, v)| v.next().is_none())
            .filter_map(|(_k, v)| *v.previous())
    }

    pub(crate) fn dropped_tables(&self) -> impl Iterator<Item = TableWalker<'a>> + '_ {
        self.tables
            .values()
            .filter(|p| p.next().is_none())
            .filter_map(|p| *p.previous())
            .map(move |idx| self.schemas.previous().table_walker_at(idx))
    }

    fn range_columns(
        &self,
        table: Pair<usize>,
    ) -> impl Iterator<Item = (&(Pair<usize>, &'a str), &Pair<Option<usize>>)> {
        self.columns
            .range((Bound::Included(&(table, "")), Bound::Unbounded))
            .take_while(move |((t, _), _)| *t == table)
    }

    /// An iterator over the tables that are present in both schemas.
    pub(crate) fn table_pairs(&self) -> impl Iterator<Item = Pair<TableWalker<'a>>> + '_ {
        self.tables
            .values()
            .filter_map(|p| p.transpose())
            .map(move |idxs| self.schemas.tables(&idxs))
    }
}
