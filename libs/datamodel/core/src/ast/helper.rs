pub fn get_sort_index_of_attribute(is_field_attribute: bool, attribute_name: &str) -> usize {
    // this must match the order defined for rendering in libs/datamodel/core/src/transform/attributes/mod.rs
    let correct_order = if is_field_attribute {
        vec!["id", "unique", "default", "updatedAt", "map", "relation"]
    } else {
        vec!["id", "unique", "index", "map"]
    };
    if let Some(sort_index) = correct_order
        .iter()
        .position(|p| attribute_name.starts_with(p) || attribute_name.starts_with(&format!("@@{}", p)))
    {
        sort_index
    } else {
        usize::MAX
    }
}
