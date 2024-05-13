use query_structure::QueryArguments;

pub(crate) trait QueryArgumentsExt {
    /// If we need to take rows before a cursor position, then we need to reverse the order in SQL.
    fn needs_reversed_order(&self) -> bool;

    /// Checks whether any form of memory processing is needed, or we could just return the records
    /// as they are. This is useful to avoid turning an existing collection of records into an
    /// iterator and re-collecting it back with no changes.
    #[cfg(feature = "relation_joins")]
    fn needs_inmemory_processing_with_joins(&self) -> bool;
}

impl QueryArgumentsExt for QueryArguments {
    fn needs_reversed_order(&self) -> bool {
        self.take.map(|t| t < 0).unwrap_or(false)
    }

    #[cfg(feature = "relation_joins")]
    fn needs_inmemory_processing_with_joins(&self) -> bool {
        self.needs_reversed_order()
            || self.requires_inmemory_distinct_with_joins()
            || self.requires_inmemory_pagination_with_joins()
    }
}
