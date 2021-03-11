use super::*;
use crate::constants::outputs;

pub fn extract_nested_rel_aggr_selections(field_pairs: Vec<FieldPair>) -> (Vec<FieldPair>, Vec<FieldPair>) {
    field_pairs
        .into_iter()
        .partition(|pair| pair.parsed_field.name == outputs::fields::UNDERSCORE_COUNT)
}
