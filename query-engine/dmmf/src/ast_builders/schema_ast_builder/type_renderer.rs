use super::{DmmfTypeReference, RenderContext, TypeLocation};
use schema::{InputType, IntoArc, OutputType, ScalarType};

pub(super) fn render_output_type(output_type: &OutputType, ctx: &mut RenderContext) -> DmmfTypeReference {
    match output_type {
        OutputType::Object(ref obj) => {
            ctx.mark_to_be_rendered(obj);

            let obj = obj.into_arc();
            let type_reference = DmmfTypeReference {
                typ: obj.identifier.name().to_string(),
                namespace: Some(obj.identifier.namespace().to_string()),
                location: TypeLocation::OutputObjectTypes,
                is_list: false,
            };

            type_reference
        }

        OutputType::Enum(et) => {
            ctx.mark_to_be_rendered(&et.as_ref());
            let type_reference = DmmfTypeReference {
                typ: et.name().to_owned(),
                namespace: Some(et.namespace()),
                location: TypeLocation::EnumTypes,
                is_list: false,
            };

            type_reference
        }

        OutputType::List(ref l) => {
            let mut type_reference = render_output_type(l, ctx);
            type_reference.is_list = true;

            type_reference
        }

        OutputType::Scalar(ScalarType::Enum(et)) => {
            ctx.mark_to_be_rendered(&et.as_ref());
            let type_reference = DmmfTypeReference {
                typ: et.name().to_owned(),
                namespace: Some(et.namespace()),
                location: TypeLocation::Scalar,
                is_list: false,
            };

            type_reference
        }

        OutputType::Scalar(ref scalar) => {
            let stringified = match scalar {
                ScalarType::Null => "Null",
                ScalarType::String => "String",
                ScalarType::Int => "Int",
                ScalarType::BigInt => "BigInt",
                ScalarType::Boolean => "Boolean",
                ScalarType::Float => "Float",
                ScalarType::Decimal => "Decimal",
                ScalarType::DateTime => "DateTime",
                ScalarType::Json => "Json",
                ScalarType::UUID => "UUID",
                ScalarType::JsonList => "Json",
                ScalarType::Xml => "Xml",
                ScalarType::Bytes => "Bytes",
                ScalarType::Enum(_) => unreachable!(), // Handled separately above.
            };

            DmmfTypeReference {
                typ: stringified.into(),
                namespace: None,
                location: TypeLocation::Scalar,
                is_list: false,
            }
        }
    }
}

pub(super) fn render_input_types(input_types: &[InputType], ctx: &mut RenderContext) -> Vec<DmmfTypeReference> {
    input_types
        .iter()
        .map(|input_type| render_input_type(input_type, ctx))
        .collect()
}

pub(super) fn render_input_type(input_type: &InputType, ctx: &mut RenderContext) -> DmmfTypeReference {
    match input_type {
        InputType::Object(ref obj) => {
            ctx.mark_to_be_rendered(obj);

            let obj = obj.into_arc();
            let type_reference = DmmfTypeReference {
                typ: obj.identifier.name().to_owned(),
                namespace: Some(obj.identifier.namespace().to_owned()),
                location: TypeLocation::InputObjectTypes,
                is_list: false,
            };

            type_reference
        }

        InputType::Enum(et) => {
            ctx.mark_to_be_rendered(&et.as_ref());
            let type_reference = DmmfTypeReference {
                typ: et.name().to_owned(),
                namespace: Some(et.namespace()),
                location: TypeLocation::EnumTypes,
                is_list: false,
            };

            type_reference
        }

        InputType::List(ref l) => {
            let mut type_reference = render_input_type(l, ctx);
            type_reference.is_list = true;

            type_reference
        }

        InputType::Scalar(ScalarType::Enum(et)) => {
            ctx.mark_to_be_rendered(&et.as_ref());
            let type_reference = DmmfTypeReference {
                typ: et.name().to_owned(),
                namespace: Some(et.namespace()),
                location: TypeLocation::Scalar,
                is_list: false,
            };

            type_reference
        }

        InputType::Scalar(ref scalar) => {
            let stringified = match scalar {
                ScalarType::Null => "Null",
                ScalarType::String => "String",
                ScalarType::Int => "Int",
                ScalarType::BigInt => "BigInt",
                ScalarType::Boolean => "Boolean",
                ScalarType::Float => "Float",
                ScalarType::Decimal => "Decimal",
                ScalarType::DateTime => "DateTime",
                ScalarType::Json => "Json",
                ScalarType::UUID => "UUID",
                ScalarType::JsonList => "Json",
                ScalarType::Xml => "Xml",
                ScalarType::Bytes => "Bytes",
                ScalarType::Enum(_) => unreachable!(), // Handled separately above.
            };

            DmmfTypeReference {
                typ: stringified.into(),
                namespace: None,
                location: TypeLocation::Scalar,
                is_list: false,
            }
        }
    }
}
