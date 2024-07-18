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
    model_field_name
    model_name
    model_relation_fields
    model_relation_references
    model_unique_fields
    view_as_type
    view_index_fields
    view_name
    view_relation_fields
    view_relation_references
    datasource_as_attribute
    datasource_name
}
