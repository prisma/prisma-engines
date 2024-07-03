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
    composite_from_field_type
    enum_from_field_type
    model_from_block_name
    model_from_view_type
    one_to_many_self_relation
}
