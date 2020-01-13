use sql_schema_describer::*;

pub(super) fn index_covers_fk(table: &Table, index: &Index) -> bool {
    table.foreign_keys.iter().any(|fk| fk.columns == index.columns)
}
