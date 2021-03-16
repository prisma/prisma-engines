use crate::Table;

/// Clean up tables by filtering out foreign keys that reference to non-existent
/// tables. This has been observed repeatedly in the wild, so this function
/// serves an important defensive programming role.
pub(crate) fn purge_dangling_foreign_keys(tables: &mut [Table]) {
    let dangling_fks: Vec<(usize, usize)> = tables
        .iter()
        .enumerate()
        .flat_map(|(table_idx, table)| {
            table
                .foreign_keys
                .iter()
                // Reversing the iterator is important, because we are going to
                // remove foreign keys by index. When there are multiple
                // dangling foreign keys (we have observed this in the wild),
                // removing starting from the lower index will invalidate
                // subsequent indexes, so _must_ start from the end (higher
                // indexes).
                .rev()
                .enumerate()
                .map(move |(reverse_fk_idx, fk)| (table_idx, reverse_fk_idx, fk))
                .filter(|(_table_idx, _reverse_fk_idx, fk)| {
                    !tables.iter().any(|table| table.name == fk.referenced_table)
                })
                .map(move |(table_idx, reverse_fk_idx, _)| (table_idx, (table.foreign_keys.len() - 1) - reverse_fk_idx))
        })
        .collect();

    for (table_idx, fk_idx) in dangling_fks {
        let table = &mut tables[table_idx];

        table.foreign_keys.remove(fk_idx);
    }
}
