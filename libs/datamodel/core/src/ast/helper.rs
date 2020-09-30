pub fn get_sort_index_of_directive(is_field_directive: bool, directive_name: &str) -> usize {
    // this must match the order defined for rendering in libs/datamodel/core/src/transform/directives/mod.rs
    let correct_order = if is_field_directive {
        vec!["id", "unique", "default", "updatedAt", "map", "relation"]
    } else {
        vec!["id", "unique", "index", "map"]
    };
    if let Some(sort_index) = correct_order
        .iter()
        .position(|p| directive_name.starts_with(p) || directive_name.starts_with(&format!("@@{}", p)))
    {
        sort_index
    } else {
        usize::MAX
    }
}
