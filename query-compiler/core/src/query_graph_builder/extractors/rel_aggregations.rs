use super::*;
use schema::constants::aggregations::*;

pub(crate) fn is_aggr_selection(pair: &FieldPair<'_>) -> bool {
    matches!(pair.parsed_field.name.as_str(), UNDERSCORE_COUNT)
}
