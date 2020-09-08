use super::{
    type_renderer::{render_input_types, render_output_type},
    DmmfInputField, DmmfOutputField, RenderContext,
};
use query_core::{FieldRef, InputFieldRef};

pub(super) fn render_input_field(input_field: &InputFieldRef, ctx: &mut RenderContext) -> DmmfInputField {
    let type_references = render_input_types(&input_field.field_types, ctx);
    let field = DmmfInputField {
        name: input_field.name.clone(),
        input_types: type_references,
        is_required: input_field.is_required,
        is_nullable: input_field.is_nullable,
    };

    field
}

pub(super) fn render_output_field(field: &FieldRef, ctx: &mut RenderContext) -> DmmfOutputField {
    let rendered_inputs = field.arguments.iter().map(|arg| render_input_field(arg, ctx)).collect();

    // let args = render_input_field(&field.arguments, ctx);

    let output_type = render_output_type(&field.field_type, ctx);
    let output_field = DmmfOutputField {
        name: field.name.clone(),
        args: rendered_inputs,
        output_type,
        is_required: field.is_required,
        is_nullable: field.is_nullable,
    };

    ctx.add_mapping(field.name.clone(), field.query_builder.as_ref());

    output_field
}

// fn render_arguments(args: &[Argument], ctx: &mut RenderContext) -> Vec<DmmfArgument> {
//     args.iter().map(|arg| render_argument(arg, ctx)).collect()
// }

// fn render_argument(arg: &Argument, ctx: &mut RenderContext) -> DmmfArgument {
//     let input_type = render_input_type(&arg.argument_type, ctx);
//     let rendered_arg = DmmfArgument {
//         name: arg.name.clone(),
//         input_type,
//     };

//     rendered_arg
// }
