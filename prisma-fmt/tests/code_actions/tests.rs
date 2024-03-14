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
    relation_mode_prisma_missing_index
    relation_mode_referential_integrity
    relation_mode_mysql_foreign_keys_set_default
    multi_schema_one_model
    multi_schema_one_model_one_enum
    multi_schema_two_models
    multi_schema_add_to_existing_schemas
    multi_schema_add_to_nonexisting_schemas
    mongodb_at_map
    mongodb_at_map_with_validation_errors
    mongodb_auto_native
}
