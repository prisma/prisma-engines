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
  empty_schema
  default_map_mssql
  no_default_map_on_postgres
  referential_actions_mssql
  referential_actions_in_progress
}
