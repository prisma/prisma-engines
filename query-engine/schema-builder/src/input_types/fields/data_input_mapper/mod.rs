mod create;
mod update;

pub(crate) use create::*;
pub(crate) use update::*;

use super::*;
use crate::constants::json_null;
use prisma_models::prelude::*;

// Todo: This isn't final, this is only the first draft to get structure into the
// wild cross-dependency waste that was the create/update inputs.
pub(crate) trait DataInputFieldMapper {
    fn map_all(&self, ctx: &mut BuilderContext, fields: &[Field]) -> Vec<InputField> {
        fields
            .iter()
            .map(|field| match field {
                Field::Scalar(sf) if sf.is_list() => self.map_scalar_list(ctx, sf),
                Field::Scalar(sf) => self.map_scalar(ctx, sf),
                Field::Relation(rf) => self.map_relation(ctx, rf),
                Field::Composite(cf) => self.map_composite(ctx, cf),
            })
            .collect()
    }

    fn map_scalar(&self, ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputField;

    fn map_scalar_list(&self, ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputField;

    fn map_relation(&self, ctx: &mut BuilderContext, rf: &RelationFieldRef) -> InputField;

    fn map_composite(&self, ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputField;
}

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
