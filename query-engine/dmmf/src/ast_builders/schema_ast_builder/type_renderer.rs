use super::{DmmfTypeReference, RenderContext, TypeLocation};
use schema::{InputType, ObjectTag, OutputType, ScalarType};

pub(super) fn render_output_type(output_type: &OutputType, ctx: &mut RenderContext) -> DmmfTypeReference {
    match output_type {
        OutputType::Object(ref obj) => {
            ctx.mark_to_be_rendered(obj);
            let obj = &ctx.query_schema.db[*obj];

            let type_reference = DmmfTypeReference {
                typ: obj.identifier.name(),
                namespace: Some(obj.identifier.namespace().to_string()),
                location: TypeLocation::OutputObjectTypes,
                is_list: false,
            };

            type_reference
        }

        OutputType::Enum(et) => {
            let et = &ctx.query_schema.db[*et];
            ctx.mark_to_be_rendered(&et);

            let ident = et.identifier();
            let type_reference = DmmfTypeReference {
                typ: ident.name(),
                namespace: Some(ident.namespace().to_owned()),
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
            let obj = &ctx.query_schema.db[*obj];

            let location = match &obj.tag {
                Some(ObjectTag::FieldRefType(_)) => TypeLocation::FieldRefTypes,
                _ => TypeLocation::InputObjectTypes,
            };

            let type_reference = DmmfTypeReference {
                typ: obj.identifier.name(),
                namespace: Some(obj.identifier.namespace().to_owned()),
                location,
                is_list: false,
            };

            type_reference
        }

        InputType::Enum(et) => {
            let et = &ctx.query_schema.db[*et];
            ctx.mark_to_be_rendered(&et);

            let ident = et.identifier();
            let type_reference = DmmfTypeReference {
                typ: ident.name(),
                namespace: Some(ident.namespace().to_owned()),
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

        InputType::Scalar(ref scalar) => {
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
