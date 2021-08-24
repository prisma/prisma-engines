pub(super) mod connect_or_create_objects;
pub(super) mod create_many_objects;
pub(super) mod create_one_objects;
pub(super) mod filter_objects;
pub(super) mod order_by_objects;
pub(super) mod update_many_objects;
pub(super) mod update_one_objects;
pub(super) mod upsert_objects;

use crate::constants::json_null;

use super::*;
use prisma_models::{RelationFieldRef, ScalarFieldRef};

fn json_null_input_enum(nullable: bool) -> EnumTypeRef {
    if nullable {
        Arc::new(string_enum_type(
            json_null::NULLABLE_INPUT_ENUM_NAME,
            vec![json_null::DB_NULL.to_owned(), json_null::JSON_NULL.to_owned()],
        ))
    } else {
        Arc::new(string_enum_type(
            json_null::INPUT_ENUM_NAME,
            vec![json_null::JSON_NULL.to_owned()],
        ))
    }
}
