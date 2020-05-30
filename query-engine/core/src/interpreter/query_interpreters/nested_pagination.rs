use connector::QueryArguments;
use prisma_models::{ManyRecords, RecordProjection};

pub struct NestedPagination {
    skip: Option<i64>,
    take: Option<i64>,
    needs_reversing: bool,
}

impl NestedPagination {
    pub fn new_from_query_args(args: &QueryArguments) -> NestedPagination {
        NestedPagination {
            skip: args.skip.clone(),
            take: args.take_abs(),
            needs_reversing: args.needs_reversed_order(),
        }
    }

    pub fn apply_pagination(&self, many_records: &mut ManyRecords) {
        if !self.must_apply_pagination() {
            return;
        }

        // replacement for SQL order by
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

        if self.needs_reversing {
            many_records.records.reverse();
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
    }

    fn must_apply_pagination(&self) -> bool {
        self.take.or(self.skip).is_some()
    }
}
