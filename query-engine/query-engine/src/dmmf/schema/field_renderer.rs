use super::{
    type_renderer::{render_input_type, render_output_type},
    DmmfArgument, DmmfInputField, DmmfOutputField, RenderContext,
};
use query_core::{Argument, FieldRef, InputFieldRef};

pub(super) fn render_input_field(input_field: &InputFieldRef, ctx: &mut RenderContext) -> DmmfInputField {
    let type_info = render_input_type(&input_field.field_type, ctx);
    let field = DmmfInputField {
        name: input_field.name.clone(),
        input_type: type_info,
    };

    field
}

pub(super) fn render_output_field(field: &FieldRef, ctx: &mut RenderContext) -> DmmfOutputField {
    let args = render_arguments(&field.arguments, ctx);
    let output_type = render_output_type(&field.field_type, ctx);
    let output_field = DmmfOutputField {
        name: field.name.clone(),
        args,
        output_type,
    };

    ctx.add_mapping(field.name.clone(), field.query_builder.as_ref());

    output_field
}

fn render_arguments(args: &[Argument], ctx: &mut RenderContext) -> Vec<DmmfArgument> {
    args.iter().map(|arg| render_argument(arg, ctx)).collect()
}

fn render_argument(arg: &Argument, ctx: &mut RenderContext) -> DmmfArgument {
    let input_type = render_input_type(&arg.argument_type, ctx);
    let rendered_arg = DmmfArgument {
        name: arg.name.clone(),
        input_type,
    };

    rendered_arg
}
