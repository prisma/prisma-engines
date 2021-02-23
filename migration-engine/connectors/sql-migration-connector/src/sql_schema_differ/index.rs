use sql_schema_describer::walkers::{IndexWalker, TableWalker};

pub(super) fn index_covers_fk(table: &TableWalker<'_>, index: &IndexWalker<'_>) -> bool {
    table
        .foreign_keys()
        .any(|fk| fk.constrained_columns_match(index.column_indexes()))
}
