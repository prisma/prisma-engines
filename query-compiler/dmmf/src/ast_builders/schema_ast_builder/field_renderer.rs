use super::{
    DmmfInputField, DmmfOutputField, RenderContext,
    type_renderer::{render_input_types, render_output_type},
};
use schema::{InputField, InputType, OutputField, ScalarType};

pub(super) fn render_input_field<'a>(input_field: &InputField<'a>, ctx: &mut RenderContext<'a>) -> DmmfInputField {
    let type_references = render_input_types(input_field.field_types(), ctx);
    let nullable = input_field
        .field_types()
        .iter()
        .any(|typ| matches!(typ, InputType::Scalar(ScalarType::Null)));

    DmmfInputField {
        name: input_field.name.to_string(),
        input_types: type_references,
        is_required: input_field.is_required(),
        is_nullable: nullable,
        is_parameterizable: input_field.is_parameterizable(),
        requires_other_fields: input_field
            .requires_other_fields()
            .iter()
            .map(|f| f.to_string())
            .collect(),
        deprecation: None,
    }
}

pub(super) fn render_output_field<'a>(field: &OutputField<'a>, ctx: &mut RenderContext<'a>) -> DmmfOutputField {
    let rendered_inputs = field
        .arguments()
        .iter()
        .map(|arg| render_input_field(arg, ctx))
        .collect();
    let output_type = render_output_type(field.field_type(), ctx);

    let output_field = DmmfOutputField {
        name: field.name().clone().into_owned(),
        args: rendered_inputs,
        output_type,
        is_nullable: field.is_nullable,
        deprecation: None,
    };

    ctx.add_mapping(field.name().clone().into_owned(), field.query_info());

    output_field
}
