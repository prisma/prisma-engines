use connector::QueryArguments;
use prisma_models::{ManyRecords, RecordIdentifier};
use std::collections::HashMap;

pub struct NestedPagination {
    skip: Option<i64>,
    take: Option<i64>,
    needs_reversing: bool,
}

impl NestedPagination {
    pub fn new_from_query_args(args: &QueryArguments) -> NestedPagination {
        NestedPagination {
            skip: args.skip.clone(),
            take: args.last.or(args.first).clone(),
            needs_reversing: args.last.is_some(),
        }
    }

    pub fn apply_pagination(&self, many_records: &mut ManyRecords) {
        if !self.must_apply_pagination() {
            return;
        }
        let mut count_by_parent_id: HashMap<Option<RecordIdentifier>, i64> = HashMap::new();
        // replacement for SQL order by
        // TODO: this must also handle secondary order bys
        many_records.records.sort_by_key(|r| {
            let values: Vec<_> = r
                .parent_id
                .as_ref()
                .expect("parent id must be set on all records in order to paginate")
                .values()
                .collect();
            values
        });

        if self.needs_reversing {
            many_records.records.reverse();
        }

        many_records.records.retain(|record| {
            let current_count = count_by_parent_id.get(&record.parent_id).unwrap_or(&0);
            let new_count = current_count + 1;
            count_by_parent_id.insert(record.parent_id.clone(), new_count);

            let is_beyond_skip_range = match self.skip {
                None => true,
                Some(skip) => new_count > skip,
            };
            let is_within_take_range = match self.take {
                None => true,
                Some(take) => new_count <= take + self.skip.unwrap_or(0),
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
