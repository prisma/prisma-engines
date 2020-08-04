pub struct RelationNames {}

impl RelationNames {
    /// generates a name for relations that have not been explicitly named by a user
    pub fn name_for_unambiguous_relation(from: &str, to: &str) -> String {
        if from < to {
            format!("{}To{}", from, to)
        } else {
            format!("{}To{}", to, from)
        }
    }

    pub fn name_for_ambiguous_relation(from: &str, to: &str, scalar_field: &str) -> String {
        if from < to {
            format!("{}_{}To{}", from, scalar_field, to)
        } else {
            format!("{}To{}_{}", to, from, scalar_field)
        }
    }
}
