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

fn field_ref_input_object_type(ctx: &mut BuilderContext<'_>, allow_type: InputType) -> InputObjectTypeWeakRef {
    let ident = Identifier::new_prisma(field_ref_input_type_name(&allow_type));

    return_cached_input!(ctx, &ident);

    let mut object = init_input_object_type(ident.clone());
    object.set_tag(ObjectTag::FieldRefType(allow_type));

    let object = Arc::new(object);
    ctx.cache_input_type(ident, object.clone());

    object.set_fields(vec![input_field(
        ctx,
        filters::UNDERSCORE_REF,
        InputType::string(),
        None,
    )]);

    Arc::downgrade(&object)
}

fn field_ref_input_type_name(allow_type: &InputType) -> String {
    let typ_str = match allow_type {
        InputType::Scalar(scalar) => match scalar {
            ScalarType::Null => unreachable!("ScalarType::Null should never reach that code path"),
            _ => scalar.to_string(),
        },
        InputType::Enum(e) => format!("Enum{}", e.into_arc().name()),
        InputType::List(inner) => return format!("List{}", field_ref_input_type_name(inner)),
        _ => unreachable!("input ref type only support scalar or enums"),
    };

    format!("{typ_str}FieldRefInput")
}
