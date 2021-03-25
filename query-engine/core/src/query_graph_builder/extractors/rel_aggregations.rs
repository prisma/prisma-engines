use super::*;
use crate::constants::outputs;

pub fn extract_nested_rel_aggr_selections(field_pairs: Vec<FieldPair>) -> (Vec<FieldPair>, Vec<FieldPair>) {
    field_pairs.into_iter().partition(is_aggr_selection)
}

fn is_aggr_selection(pair: &FieldPair) -> bool {
    match pair.parsed_field.name.as_str() {
        outputs::fields::UNDERSCORE_COUNT => true,
        _ => false,
    }
}
