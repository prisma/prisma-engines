use std::collections::HashSet;

use crate::query_arguments_ext::QueryArgumentsExt;
use query_structure::{ManyRecords, QueryArguments};

pub struct InMemoryProcessorForJoins<'a> {
    args: &'a QueryArguments,
}

trait ApplyReverseOrder {
    fn apply_reverse_order(&mut self, args: &QueryArguments);
}

trait ApplyDistinct {
    fn apply_distinct(&mut self, args: &QueryArguments);
}

trait ApplyPagination {
    fn apply_pagination(&mut self, args: &QueryArguments);
}

impl<T> ApplyReverseOrder for Vec<T> {
    fn apply_reverse_order(&mut self, args: &QueryArguments) {
        if args.needs_reversed_order() {
            self.reverse();
        }
    }
}

impl ApplyDistinct for ManyRecords {
    fn apply_distinct(&mut self, args: &QueryArguments) {
        if let Some(distinct) = args.distinct.as_ref() {
            if args.requires_inmemory_distinct_with_joins() {
                let mut seen = HashSet::new();

                self.records.retain(|record| {
                    seen.insert(
                        record
                            .extract_selection_result_from_prisma_name(&self.field_names, distinct)
                            .unwrap(),
                    )
                });
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum ScalarJsonValue {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<ScalarJsonValue>),
}

fn extract_json_scalars(value: &serde_json::Value) -> ScalarJsonValue {
    match value {
        serde_json::Value::Null => ScalarJsonValue::Null,
        serde_json::Value::Bool(b) => ScalarJsonValue::Bool(*b),
        serde_json::Value::Number(n) => ScalarJsonValue::Number(n.clone()),
        serde_json::Value::String(s) => ScalarJsonValue::String(s.clone()),
        serde_json::Value::Array(arr) => ScalarJsonValue::Array(arr.iter().map(extract_json_scalars).collect()),
        _ => unreachable!(),
    }
}

impl ApplyDistinct for Vec<serde_json::Value> {
    fn apply_distinct(&mut self, args: &QueryArguments) {
        if let Some(distinct) = args.distinct.as_ref() {
            if args.requires_inmemory_distinct_with_joins() {
                let mut seen = HashSet::new();

                self.retain(|record| {
                    let extracted_values = distinct
                        .selections()
                        .map(|sf| {
                            (
                                sf.prisma_name().into_owned(),
                                extract_json_scalars(
                                    record.as_object().unwrap().get(sf.prisma_name().as_ref()).unwrap(),
                                ),
                            )
                        })
                        .collect::<Vec<_>>();

                    seen.insert(extracted_values)
                });
            }
        }
    }
}

impl<T> ApplyPagination for Vec<T> {
    fn apply_pagination(&mut self, args: &QueryArguments) {
        if let Some(skip) = args.skip {
            if args.requires_inmemory_pagination_with_joins() {
                self.drain(0..std::cmp::min(skip as usize, self.len()));
            }
        }

        if let Some(take) = args.take_abs() {
            if args.requires_inmemory_pagination_with_joins() {
                self.truncate(std::cmp::min(take as usize, self.len()));
            }
        }
    }
}

impl<'a> InMemoryProcessorForJoins<'a> {
    pub fn new(args: &'a QueryArguments) -> Self {
        Self { args }
    }

    pub fn process_records(&mut self, records: &mut ManyRecords) {
        records.records.apply_reverse_order(self.args);
        records.apply_distinct(self.args);
        records.records.apply_pagination(self.args);
    }

    pub fn process_json_values(&mut self, records: &mut Vec<serde_json::Value>) {
        records.apply_reverse_order(self.args);
        records.apply_distinct(self.args);
        records.apply_pagination(self.args);
    }
}
