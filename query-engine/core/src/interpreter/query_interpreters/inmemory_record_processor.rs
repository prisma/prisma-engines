use itertools::Itertools;
use query_structure::*;
use std::ops::Deref;

#[derive(Debug)]
/// Allows to manipulate a set of records in-memory instead of on the database level.
pub struct InMemoryRecordProcessor {
    args: QueryArguments,
}

impl Deref for InMemoryRecordProcessor {
    type Target = QueryArguments;

    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

impl InMemoryRecordProcessor {
    /// Creates a new processor from the given query args.
    /// The original args will be modified to prevent db level processing.
    pub(crate) fn new_from_query_args(args: &mut QueryArguments) -> Self {
        let mut processor = Self { args: args.clone() };

        if !args.requires_inmemory_distinct() {
            processor.args.distinct = None;
        } else {
            args.distinct = None;
        }

        args.ignore_take = true;
        args.ignore_skip = true;

        processor
    }

    /// Checks whether or not we need to take records going backwards in the record list,
    /// which requires reversing the list of records at some point.
    fn needs_reversed_order(&self) -> bool {
        self.take.map(|t| t < 0).unwrap_or(false)
    }

    pub(crate) fn apply(&self, mut records: ManyRecords) -> ManyRecords {
        if self.needs_reversed_order() {
            records.reverse();
        }

        let records = if Self::is_nested(&records) {
            Self::order_by_parent(records)
        } else {
            records
        };

        let records = if self.requires_inmemory_distinct() {
            self.apply_distinct(records)
        } else {
            records
        };

        let mut records = self.apply_pagination(records);

        if self.needs_reversed_order() {
            records.reverse();
        }

        records
    }

    fn order_by_parent(mut records: ManyRecords) -> ManyRecords {
        records.records.sort_by_key(|r| {
            let values: Vec<_> = r
                .parent_id
                .as_ref()
                .expect("Expected parent IDs to be set when ordering by parent ID.")
                .values()
                .collect();

            values
        });

        records
    }

    fn is_nested(records: &ManyRecords) -> bool {
        records.records.first().map(|x| x.parent_id.is_some()).unwrap_or(false)
    }

    fn apply_distinct(&self, mut records: ManyRecords) -> ManyRecords {
        let field_names = &records.field_names;

        let distinct_selection = if let Some(ref distinct) = self.distinct {
            distinct.clone()
        } else {
            return records;
        };

        let new_records: Vec<Record> = if Self::is_nested(&records) {
            records
                .records
                .into_iter()
                .group_by(|record| record.parent_id.clone())
                .into_iter()
                .flat_map(|(_, group)| {
                    let filtered: Vec<_> = group
                        .into_iter()
                        .unique_by(|record| {
                            record
                                .extract_selection_result_from_db_name(field_names, &distinct_selection)
                                .unwrap()
                        })
                        .collect();

                    filtered
                })
                .collect()
        } else {
            records
                .records
                .into_iter()
                .unique_by(|record| {
                    record
                        .extract_selection_result_from_db_name(field_names, &distinct_selection)
                        .unwrap()
                })
                .collect()
        };

        records.records = new_records;
        records
    }

    fn apply_pagination(&self, mut many_records: ManyRecords) -> ManyRecords {
        if !self.must_apply_pagination() {
            return many_records;
        }

        // If we have a cursor, skip records until we find it for each parent id. Pagination is applied afterwards.
        if let Some(cursor) = &self.cursor {
            let cursor_values: Vec<_> = cursor.values().collect();
            let cursor_selection: FieldSelection = cursor.into();
            let field_names = &many_records.field_names;

            let mut current_parent_id = None;
            let mut cursor_seen = false;

            many_records.records.retain(|record| {
                let cursor_comparator = record
                    .extract_selection_result_from_db_name(field_names, &cursor_selection)
                    .unwrap();
                let record_values: Vec<_> = cursor_comparator.values().collect();

                // Reset, new parent
                if current_parent_id != record.parent_id {
                    current_parent_id.clone_from(&record.parent_id);
                    cursor_seen = false;
                }

                // As long as the cursor has not been seen we recheck every record.
                if !cursor_seen {
                    cursor_seen = record_values == cursor_values;
                }

                // If the cursor has been seen for this parent, we retain all records coming afterwards (and including the cursor).
                cursor_seen
            });
        }

        // The records are sorted by their parent id. Hence we just need to remember the count for the last parent id to apply pagination.
        let mut current_count: i64 = 0;
        let mut last_parent_id: Option<SelectionResult> = None;

        many_records.records.retain(|record| {
            if last_parent_id == record.parent_id {
                current_count += 1;
            } else {
                last_parent_id.clone_from(&record.parent_id);
                current_count = 1; // this is the first record we see for this parent id
            };

            let is_beyond_skip_range = match self.skip {
                None => true,
                Some(skip) => current_count > skip,
            };
            let is_within_take_range = match self.take_abs() {
                None => true,
                Some(take) => current_count <= take + self.skip.unwrap_or(0),
            };

            is_beyond_skip_range && is_within_take_range
        });

        many_records
    }

    fn must_apply_pagination(&self) -> bool {
        self.take.or(self.skip).is_some() || self.cursor.is_some()
    }
}
