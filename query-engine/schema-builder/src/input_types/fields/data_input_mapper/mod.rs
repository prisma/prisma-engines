mod create;
mod update;

pub(crate) use create::*;
pub(crate) use update::*;

use super::*;
use prisma_models::prelude::*;

// Todo: This isn't final, this is only the first draft to get structure into the
// wild cross-dependency waste that was the create/update inputs.
pub(crate) trait DataInputFieldMapper {
    fn map_field(&self, ctx: &mut BuilderContext<'_>, id: InputObjectTypeId, field: &Field) {
        let field = match field {
            Field::Scalar(sf) if sf.is_list() => self.map_scalar_list(ctx, sf),
            Field::Scalar(sf) => self.map_scalar(ctx, sf),
            Field::Relation(rf) => self.map_relation(ctx, rf),
            Field::Composite(cf) => self.map_composite(ctx, cf),
        };
        ctx.db.push_input_field(id, field);
    }

    fn map_all(&self, ctx: &mut BuilderContext<'_>, id: InputObjectTypeId, fields: &mut dyn Iterator<Item = &Field>) {
        for field in fields {
            self.map_field(ctx, id, field);
        }
    }

    fn map_scalar(&self, ctx: &mut BuilderContext<'_>, sf: &ScalarFieldRef) -> InputField;

    fn map_scalar_list(&self, ctx: &mut BuilderContext<'_>, sf: &ScalarFieldRef) -> InputField;

    fn map_relation(&self, ctx: &mut BuilderContext<'_>, rf: &RelationFieldRef) -> InputField;

    fn map_composite(&self, ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> InputField;
}
