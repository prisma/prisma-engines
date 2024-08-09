//! Test suite for mongodb `db push`.
//!
//! For each test, create a folder under `scenarios` and add it to the list below.
//!
//! Each test scenario folder must contain two files:
//!
//! - `state.json` must contain the initial state of the database. See examples and `State` in
//!   `test_api.rs` for details.
//! - `schema.prisma` must be the Prisma schema.
//!
//! On the first run, a `result` file will also be created. It is a snapshot test, do not edit it
//! manually.

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
  indexes_can_be_created
  indexes_can_be_created_descending
  indexes_can_be_changed_from_descending_to_ascending
  index_sort_order_doesnt_count_without_preview_feature
  indexes_can_be_dropped
  indexes_can_be_renamed
  indexes_on_nested_fields_get_dropped // https://docs.mongodb.com/manual/core/index-multikey/ - not supported yet
  index_keys_can_be_changed
  index_to_unique
  map_annotations
  single_field_uniques_are_created
  single_column_fulltext_indexes_can_be_created
  multi_column_fulltext_indexes_can_be_created
  multi_column_mixed_fulltext_indexes_can_be_created
  multi_column_mixed_fulltext_indexes_can_be_changed
  pushing_without_preview_flag_should_not_touch_fts_indexes
  unique_to_index
  composite_indexes_can_be_created
  composite_indexes_can_be_created_descending
  composite_indexes_can_be_changed_from_descending_to_ascending
  composite_indexes_can_be_dropped
  composite_indexes_can_be_renamed
  composite_indexes_work_on_arrays
}
