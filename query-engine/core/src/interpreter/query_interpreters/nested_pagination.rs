use connector::QueryArguments;
use prisma_models::{ManyRecords, ModelProjection, RecordProjection};

pub struct NestedPagination {
    skip: Option<i64>,
    take: Option<i64>,
    cursor: Option<RecordProjection>,
    needs_reversing: bool,
}

impl NestedPagination {
    pub fn new_from_query_args(args: &QueryArguments) -> NestedPagination {
        NestedPagination {
            skip: args.skip.clone(),
            take: args.take_abs(),
            cursor: args.cursor.clone(),
            needs_reversing: args.needs_reversed_order(),
        }
    }

    pub fn apply_pagination(&self, mut many_records: ManyRecords) -> ManyRecords {
        if !self.must_apply_pagination() {
            return many_records;
        }

        if self.needs_reversing {
            many_records.records.reverse();
        }

        // Replacement for SQL order by
        // TODO: this must also handle secondary order bys
        many_records.records.sort_by_key(|r| {
            let values: Vec<_> = r
                .parent_id
                .as_ref()
                .expect("Parent id must be set on all records in order to paginate")
                .values()
                .collect();
            values
        });

        // If we have a cursor, skip records until we find it for each parent id. Pagination is applied afterwards.
        if let Some(cursor) = &self.cursor {
            let cursor_values: Vec<_> = cursor.values().collect();
            let cursor_projection: ModelProjection = cursor.into();
            let field_names = &many_records.field_names;

            let mut current_parent_id = None;
            let mut cursor_seen = false;

            many_records.records.retain(|record| {
                let cursor_comparator = record.projection(field_names, &cursor_projection).unwrap();
                let record_values: Vec<_> = cursor_comparator.values().collect();

                // Reset, new parent
                if current_parent_id != record.parent_id {
                    current_parent_id = record.parent_id.clone();
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
        let mut last_parent_id: Option<RecordProjection> = None;

        many_records.records.retain(|record| {
            if last_parent_id == record.parent_id {
                current_count = current_count + 1;
            } else {
                last_parent_id = record.parent_id.clone();
                current_count = 1; // this is the first record we see for this parent id
            };

            let is_beyond_skip_range = match self.skip {
                None => true,
                Some(skip) => current_count > skip,
            };
            let is_within_take_range = match self.take {
                None => true,
                Some(take) => current_count <= take + self.skip.unwrap_or(0),
            };

            is_beyond_skip_range && is_within_take_range
        });

        if self.needs_reversing {
            many_records.records.reverse();
        }

        many_records
    }

    fn must_apply_pagination(&self) -> bool {
        self.take.or(self.skip).is_some() || self.cursor.is_some()
    }
}
