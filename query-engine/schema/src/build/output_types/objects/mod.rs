pub mod composite;
pub mod model;

use super::*;
use constants::output_fields::*;

pub(crate) fn affected_records_object_type<'a>() -> ObjectType<'a> {
    ObjectType::new(Identifier::new_prisma(IdentifierType::AffectedRowsOutput), || {
        vec![field(
            AFFECTED_COUNT,
            None,
            OutputType::non_list(OutputType::int()),
            None,
        )]
    })
}
