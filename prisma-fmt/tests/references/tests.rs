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
    composite_type_as_type
    composite_type_name
    enum_as_type
    enum_name
    model_as_type
    model_name
    model_relation_fields
    model_relation_references
    view_as_type
    view_name
    datasource_as_attribute
    datasource_name
}
