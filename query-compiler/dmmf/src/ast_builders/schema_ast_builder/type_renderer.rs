use super::{DmmfTypeReference, RenderContext, TypeLocation};
use schema::{InnerOutputType, InputType, ObjectTag, OutputType, ScalarType};

pub(super) fn render_output_type<'a>(output_type: &OutputType<'a>, ctx: &mut RenderContext<'a>) -> DmmfTypeReference {
    match &output_type.inner {
        _ if output_type.is_list() => {
            let mut type_reference = render_output_type(&OutputType::non_list(output_type.inner.clone()), ctx);
            type_reference.is_list = true;
            type_reference
        }
        InnerOutputType::Object(obj) => {
            ctx.mark_to_be_rendered(obj);

            DmmfTypeReference {
                typ: obj.name(),
                namespace: Some(obj.identifier().namespace().to_string()),
                location: TypeLocation::OutputObjectTypes,
                is_list: false,
            }
        }

        InnerOutputType::Enum(et) => {
            ctx.mark_to_be_rendered(et);

            let ident = et.identifier();

            DmmfTypeReference {
                typ: ident.name(),
                namespace: Some(ident.namespace().to_owned()),
                location: TypeLocation::EnumTypes,
                is_list: false,
            }
        }

        InnerOutputType::Scalar(scalar) => {
            let stringified: String = match scalar {
                ScalarType::Null => "Null".into(),
                ScalarType::String => "String".into(),
                ScalarType::Int => "Int".into(),
                ScalarType::BigInt => "BigInt".into(),
                ScalarType::Boolean => "Boolean".into(),
                ScalarType::Float => "Float".into(),
                ScalarType::Decimal => "Decimal".into(),
                ScalarType::DateTime => "DateTime".into(),
                ScalarType::Json => "Json".into(),
                ScalarType::UUID => "UUID".into(),
                ScalarType::JsonList => "Json".into(),
                ScalarType::Bytes => "Bytes".into(),
                ScalarType::Geometry(s) => s.clone(),
            };

            DmmfTypeReference {
                typ: stringified,
                namespace: None,
                location: TypeLocation::Scalar,
                is_list: false,
            }
        }
    }
}

pub(super) fn render_input_types<'a>(
    input_types: &[InputType<'a>],
    ctx: &mut RenderContext<'a>,
) -> Vec<DmmfTypeReference> {
    input_types
        .iter()
        .map(|input_type| render_input_type(input_type, ctx))
        .collect()
}

pub(super) fn render_input_type<'a>(input_type: &InputType<'a>, ctx: &mut RenderContext<'a>) -> DmmfTypeReference {
    match input_type {
        InputType::Object(obj) => {
            ctx.mark_to_be_rendered(obj);

            let location = match obj.tag() {
                Some(ObjectTag::FieldRefType(_)) => TypeLocation::FieldRefTypes,
                _ => TypeLocation::InputObjectTypes,
            };

            DmmfTypeReference {
                typ: obj.identifier.name(),
                namespace: Some(obj.identifier.namespace().to_owned()),
                location,
                is_list: false,
            }
        }

        InputType::Enum(et) => {
            ctx.mark_to_be_rendered(et);

            let ident = et.identifier();

            DmmfTypeReference {
                typ: ident.name(),
                namespace: Some(ident.namespace().to_owned()),
                location: TypeLocation::EnumTypes,
                is_list: false,
            }
        }

        InputType::List(l) => {
            let mut type_reference = render_input_type(l, ctx);
            type_reference.is_list = true;

            type_reference
        }

        InputType::Scalar(scalar) => {
            let stringified = scalar.to_string();

            DmmfTypeReference {
                typ: stringified,
                namespace: None,
                location: TypeLocation::Scalar,
                is_list: false,
            }
        }
    }
}
