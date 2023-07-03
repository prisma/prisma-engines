use super::*;
use constants::filters;

pub(crate) trait WithFieldRefInputExt<'a> {
    fn with_field_ref_input(self) -> Vec<InputType<'a>>;
}

impl<'a> WithFieldRefInputExt<'a> for InputType<'a> {
    fn with_field_ref_input(self) -> Vec<InputType<'a>> {
        let field_types = vec![self.clone(), InputType::object(field_ref_input_object_type(self))];

        field_types
    }
}

fn field_ref_input_object_type(allow_type: InputType<'_>) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(field_ref_input_type_name(&allow_type));
    let mut object = init_input_object_type(ident);
    object.set_tag(ObjectTag::FieldRefType(Box::new(allow_type)));
    object.set_fields(|| {
        vec![
            input_field(filters::UNDERSCORE_REF, vec![InputType::string()], None),
            input_field(filters::UNDERSCORE_CONTAINER, vec![InputType::string()], None),
        ]
    });
    object
}

fn field_ref_input_type_name(allow_type: &InputType<'_>) -> String {
    let typ_str = match allow_type {
        InputType::Scalar(scalar) => match scalar {
            ScalarType::Null => unreachable!("ScalarType::Null should never reach that code path"),
            _ => scalar.to_string(),
        },
        InputType::Enum(e) => format!("Enum{}", e.name()),
        InputType::List(inner) => return format!("List{}", field_ref_input_type_name(inner)),
        _ => unreachable!("input ref type only support scalar or enums"),
    };

    format!("{typ_str}FieldRefInput")
}
