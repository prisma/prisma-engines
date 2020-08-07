use super::{DMMFTypeInfo, RenderContext, TypeKind};
use query_core::{InputType, IntoArc, OutputType, ScalarType};

// WIP dedup code
pub(super) fn render_output_type(output_type: &OutputType, ctx: &mut RenderContext) -> DMMFTypeInfo {
    match output_type {
        OutputType::Object(ref obj) => {
            ctx.mark_to_be_rendered(obj);
            let type_info = DMMFTypeInfo {
                typ: obj.into_arc().name().to_string(),
                kind: TypeKind::Object,
                is_required: true,
                is_list: false,
                is_nullable: false,
            };

            type_info
        }
        OutputType::Enum(et) => {
            ctx.mark_to_be_rendered(&et.as_ref());
            let type_info = DMMFTypeInfo {
                typ: et.name().to_owned(),
                kind: TypeKind::Enum,
                is_required: true,
                is_list: false,
                is_nullable: false,
            };

            type_info
        }
        OutputType::List(ref l) => {
            let mut type_info = render_output_type(l, ctx);
            type_info.is_list = true;

            type_info
        }
        OutputType::Opt(ref opt) => {
            let mut type_info = render_output_type(opt, ctx);
            type_info.is_required = false;

            type_info
        }
        OutputType::Scalar(ScalarType::Enum(et)) => {
            ctx.mark_to_be_rendered(&et.as_ref());
            let type_info = DMMFTypeInfo {
                typ: et.name().to_owned(),
                kind: TypeKind::Scalar,
                is_required: true,
                is_list: false,
                is_nullable: false,
            };

            type_info
        }
        OutputType::Scalar(ref scalar) => {
            let stringified = match scalar {
                ScalarType::String => "String",
                ScalarType::Int => "Int",
                ScalarType::Boolean => "Boolean",
                ScalarType::Float => "Float",
                ScalarType::DateTime => "DateTime",
                ScalarType::Json => "Json",
                ScalarType::UUID => "UUID",
                ScalarType::JsonList => "Json",
                ScalarType::Enum(_) => unreachable!(), // Handled separately above.
            };

            let type_info = DMMFTypeInfo {
                typ: stringified.into(),
                kind: TypeKind::Scalar,
                is_required: true,
                is_list: false,
                is_nullable: false,
            };

            type_info
        }
    }
}

pub(super) fn render_input_type(input_type: &InputType, ctx: &mut RenderContext) -> DMMFTypeInfo {
    match input_type {
        InputType::Object(ref obj) => {
            ctx.mark_to_be_rendered(obj);
            let type_info = DMMFTypeInfo {
                typ: obj.into_arc().name.clone(),
                kind: TypeKind::Object,
                is_required: true,
                is_list: false,
                is_nullable: false,
            };

            type_info
        }

        InputType::Enum(et) => {
            ctx.mark_to_be_rendered(&et.as_ref());
            let type_info = DMMFTypeInfo {
                typ: et.name().to_owned(),
                kind: TypeKind::Enum,
                is_required: true,
                is_list: false,
                is_nullable: false,
            };

            type_info
        }

        InputType::List(ref l) => {
            let mut type_info = render_input_type(l, ctx);
            type_info.is_list = true;

            type_info
        }

        InputType::Opt(ref inner) => {
            let mut type_info = render_input_type(inner, ctx);
            type_info.is_required = false;

            type_info
        }

        InputType::Null(ref inner) => {
            let mut type_info = render_input_type(inner, ctx);
            type_info.is_nullable = true;

            type_info
        }

        InputType::Scalar(ScalarType::Enum(et)) => {
            ctx.mark_to_be_rendered(&et.as_ref());
            let type_info = DMMFTypeInfo {
                typ: et.name().to_owned(),
                kind: TypeKind::Scalar,
                is_required: true,
                is_list: false,
                is_nullable: false,
            };

            type_info
        }

        InputType::Scalar(ref scalar) => {
            let stringified = match scalar {
                ScalarType::String => "String",
                ScalarType::Int => "Int",
                ScalarType::Boolean => "Boolean",
                ScalarType::Float => "Float",
                ScalarType::DateTime => "DateTime",
                ScalarType::Json => "Json",
                ScalarType::UUID => "UUID",
                ScalarType::JsonList => "Json",
                ScalarType::Enum(_) => unreachable!(), // Handled separately above.
            };

            let type_info = DMMFTypeInfo {
                typ: stringified.into(),
                kind: TypeKind::Scalar,
                is_required: true,
                is_list: false,
                is_nullable: false,
            };

            type_info
        }
    }
}
