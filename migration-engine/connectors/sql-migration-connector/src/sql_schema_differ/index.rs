use sql_schema_describer::walkers::{IndexWalker, TableWalker};

pub(super) fn index_covers_fk(table: TableWalker<'_>, index: IndexWalker<'_>) -> bool {
    table.foreign_keys().any(|fk| {
        let fk_cols = fk.constrained_columns().map(|col| col.name());
        let index_cols = index.column_names();

        fk_cols.len() == index_cols.len() && fk_cols.zip(index_cols).all(|(a, b)| a == b)
    })
}
