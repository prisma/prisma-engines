use super::*;
use crate::constants::filters;

pub(crate) trait WithFieldRefInputExt {
    fn with_field_ref_input(self, ctx: &mut BuilderContext<'_>) -> Vec<InputType>;
}

impl WithFieldRefInputExt for InputType {
    fn with_field_ref_input(self, ctx: &mut BuilderContext<'_>) -> Vec<InputType> {
        let mut field_types: Vec<InputType> = vec![self.clone()];

        if ctx.has_feature(PreviewFeature::FieldReference) {
            field_types.push(InputType::object(field_ref_input_object_type(ctx, self)));
        }

        field_types
    }
}

fn field_ref_input_object_type(ctx: &mut BuilderContext<'_>, allow_type: InputType) -> InputObjectTypeId {
    let ident = Identifier::new_prisma(field_ref_input_type_name(&allow_type, ctx));

    return_cached_input!(ctx, &ident);

    let mut object = init_input_object_type(ident.clone());
    object.set_tag(ObjectTag::FieldRefType(allow_type));
    let id = ctx.cache_input_type(ident, object);

    let fields = vec![input_field(ctx, filters::UNDERSCORE_REF, InputType::string(), None)];
    ctx.db[id].set_fields(fields);
    id
}

fn field_ref_input_type_name(allow_type: &InputType, ctx: &mut BuilderContext<'_>) -> String {
    let typ_str = match allow_type {
        InputType::Scalar(scalar) => match scalar {
            ScalarType::Null => unreachable!("ScalarType::Null should never reach that code path"),
            _ => scalar.to_string(),
        },
        InputType::Enum(e) => format!("Enum{}", ctx.db[*e].name()),
        InputType::List(inner) => return format!("List{}", field_ref_input_type_name(inner, ctx)),
        _ => unreachable!("input ref type only support scalar or enums"),
    };

    format!("{typ_str}FieldRefInput")
}
