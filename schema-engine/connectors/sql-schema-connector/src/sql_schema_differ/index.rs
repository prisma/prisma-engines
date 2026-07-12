use either::Either;
use sql_schema_describer::{
    ForeignKeyWalker, IndexType,
    walkers::{IndexWalker, TableWalker},
};
use std::iter;

pub(super) fn get_fks_covered_by_index<'a>(
    table: TableWalker<'a>,
    index: IndexWalker<'a>,
) -> impl Iterator<Item = ForeignKeyWalker<'a>> {
    // Only normal, unique and primary key indexes can cover foreign keys.
    if !matches!(
        index.index_type(),
        IndexType::Normal | IndexType::Unique | IndexType::PrimaryKey
    ) {
        return Either::Left(iter::empty());
    }

    Either::Right(table.foreign_keys().filter(move |fk| {
        let fk_cols = fk.constrained_columns().map(|col| col.name());
        let index_cols = index.column_names();

        // It's sufficient that leftmost columns of the index match the FK columns.
        fk_cols.len() <= index_cols.len() && fk_cols.zip(index_cols).all(|(a, b)| a == b)
    }))
}
