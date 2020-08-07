use super::{DMMFArgument, DMMFField, DMMFInputField, IntoRenderer, RenderContext};
use query_core::{Argument, FieldRef, InputFieldRef};

pub(super) fn render_input_field(input_field: &InputFieldRef, ctx: &mut RenderContext) -> DMMFInputField {
    let type_info = input_field.field_type.into_renderer().render(ctx);
    let field = DMMFInputField {
        name: input_field.name.clone(),
        input_type: type_info,
    };

    field
}

pub(super) fn render_output_field(field: &FieldRef, ctx: &mut RenderContext) -> DMMFField {
    let args = render_arguments(&field.arguments, ctx);
    let output_type = field.field_type.into_renderer().render(ctx);
    let output_field = DMMFField {
        name: field.name.clone(),
        args,
        output_type,
    };

    ctx.add_mapping(field.name.clone(), field.query_builder.as_ref());

    output_field
}

fn render_arguments(args: &[Argument], ctx: &mut RenderContext) -> Vec<DMMFArgument> {
    args.iter().map(|arg| render_argument(arg, ctx)).collect()
}

fn render_argument(arg: &Argument, ctx: &mut RenderContext) -> DMMFArgument {
    let input_type = (&arg.argument_type).into_renderer().render(ctx);
    let rendered_arg = DMMFArgument {
        name: arg.name.clone(),
        input_type,
    };

    rendered_arg
}
