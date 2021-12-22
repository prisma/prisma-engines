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
    argument_after_trailing_comma
    default_map_end_of_args_list
    default_map_mssql
    empty_schema
    no_default_map_on_postgres
    referential_actions_end_of_args_list
    referential_actions_in_progress
    referential_actions_middle_of_args_list
    referential_actions_mssql
    referential_actions_with_trailing_comma
}
