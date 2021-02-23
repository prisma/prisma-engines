/// Get the sort order for an attribute, in the canonical sorting order.
pub(crate) fn get_sort_index_of_attribute(is_field_attribute: bool, attribute_name: &str) -> usize {
    // this must match the order defined for rendering in libs/datamodel/core/src/transform/attributes/mod.rs
    let correct_order: &[&str] = if is_field_attribute {
        &["id", "unique", "default", "updatedAt", "map", "relation"]
    } else {
        &["id", "unique", "index", "map"]
    };

    correct_order
        .iter()
        .position(|p| attribute_name.trim_start_matches('@').starts_with(p))
        .unwrap_or(usize::MAX)
}
