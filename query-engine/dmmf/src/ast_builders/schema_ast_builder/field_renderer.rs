use super::{
    type_renderer::{render_input_types, render_output_type},
    DmmfInputField, DmmfOutputField, RenderContext,
};
use schema::{InputFieldRef, InputType, OutputFieldRef, ScalarType};

pub(super) fn render_input_field(input_field: &InputFieldRef, ctx: &mut RenderContext) -> DmmfInputField {
    let type_references = render_input_types(&input_field.field_types, ctx);
    let nullable = input_field
        .field_types
        .iter()
        .any(|typ| matches!(typ, InputType::Scalar(ScalarType::Null)));

    let field = DmmfInputField {
        name: input_field.name.clone(),
        input_types: type_references,
        is_required: input_field.is_required,
        is_nullable: nullable,
        deprecation: input_field.deprecation.as_ref().map(Into::into),
    };

    field
}

pub(super) fn render_output_field(field: &OutputFieldRef, ctx: &mut RenderContext) -> DmmfOutputField {
    let rendered_inputs = field.arguments.iter().map(|arg| render_input_field(arg, ctx)).collect();
    let output_type = render_output_type(&field.field_type, ctx);

    let output_field = DmmfOutputField {
        name: field.name.clone(),
        args: rendered_inputs,
        output_type,
        is_nullable: field.is_nullable,
        deprecation: field.deprecation.as_ref().map(Into::into),
    };

    ctx.add_mapping(field.name.clone(), field.query_info.as_ref());

    output_field
}
