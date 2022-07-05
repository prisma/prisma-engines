use super::test_api::test_scenario;

macro_rules! scenarios {
    ($($scenario_name:ident)+) => {
        $(
            #[test]
            fn $scenario_name() {
                test_scenario(stringify!($scenario_name))
            }
        )*
    }
}

scenarios! {
    one_to_many_referenced_side_misses_unique_single_field
    one_to_many_referenced_side_misses_unique_single_field_broken_relation
    one_to_many_referenced_side_misses_unique_compound_field
    one_to_many_referenced_side_misses_unique_compound_field_existing_arguments
    one_to_many_referenced_side_misses_unique_compound_field_indentation_four_spaces
    one_to_many_referenced_side_misses_unique_compound_field_broken_relation
    one_to_one_referenced_side_misses_unique_single_field
    one_to_one_referenced_side_misses_unique_compound_field
    one_to_one_referencing_side_misses_unique_single_field
    one_to_one_referencing_side_misses_unique_compound_field
    one_to_one_referencing_side_misses_unique_compound_field_indentation_four_spaces
}
