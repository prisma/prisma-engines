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
    default_map_mssql_multifile
    empty_schema
    extended_indexes_basic
    extended_indexes_types_postgres
    extended_indexes_types_postgres_multifile
    extended_indexes_types_mysql
    extended_indexes_types_sqlserver
    extended_indexes_types_sqlite
    extended_indexes_types_mongo
    extended_indexes_types_cockroach
    extended_indexes_operators_postgres_gist
    extended_indexes_operators_postgres_gin
    extended_indexes_operators_postgres_spgist
    extended_indexes_operators_postgres_brin
    extended_indexes_operators_cockroach_gin
    language_tools_relation_directive
    no_default_map_on_postgres
    no_default_map_on_postgres_multifile
    referential_actions_multifile
    referential_actions_end_of_args_list
    referential_actions_in_progress
    referential_actions_in_progress_2
    referential_actions_middle_of_args_list
    referential_actions_mssql
    referential_actions_relation_mode_prisma_mongodb
    referential_actions_relation_mode_prisma_mysql
    referential_actions_relation_mode_fk_mysql
    referential_actions_with_trailing_comma
    datasource_default_completions
    datasource_multischema
    datasource_url_arguments
    datasource_direct_url_arguments
    datasource_shadowdb_url_arguments
    datasource_env_db_url
    datasource_env_db_direct_url
    datasource_env_db_shadowdb_url
}
