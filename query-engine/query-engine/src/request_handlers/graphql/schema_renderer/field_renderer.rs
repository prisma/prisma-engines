use super::*;

#[derive(Debug)]
pub enum GqlFieldRenderer {
    Input(InputFieldRef),
    Output(FieldRef),
}

impl Renderer for GqlFieldRenderer {
    fn render(&self, ctx: &mut RenderContext) -> String {
        match self {
            GqlFieldRenderer::Input(input) => self.render_input_field(Arc::clone(input), ctx),
            GqlFieldRenderer::Output(output) => self.render_output_field(Arc::clone(output), ctx),
        }
    }
}

impl GqlFieldRenderer {
    fn render_input_field(&self, input_field: InputFieldRef, ctx: &mut RenderContext) -> String {
        let rendered_type = (&input_field.field_type).into_renderer().render(ctx);

        format!("{}: {}", input_field.name, rendered_type)
    }

    fn render_output_field(&self, field: FieldRef, ctx: &mut RenderContext) -> String {
        let rendered_args = self.render_arguments(&field.arguments, ctx);
        let rendered_args = if rendered_args.is_empty() {
            "".into()
        } else if rendered_args.len() > 1 {
            // Multiline - double indent.
            format!(
                "({}\n{})",
                rendered_args
                    .into_iter()
                    .map(|arg| format!("\n{}{}", ctx.indent().repeat(2), arg))
                    .collect::<Vec<String>>()
                    .join(""),
                ctx.indent()
            )
        } else {
            // Single line
            format!("({})", rendered_args.join(", "))
        };

        let rendered_type = field.field_type.into_renderer().render(ctx);
        format!("{}{}: {}", field.name, rendered_args, rendered_type)
    }

    fn render_arguments(&self, args: &[Argument], ctx: &mut RenderContext) -> Vec<String> {
        let mut output = Vec::with_capacity(args.len());

        for arg in args {
            output.push(self.render_argument(arg, ctx))
        }

        output
    }

    fn render_argument(&self, arg: &Argument, ctx: &mut RenderContext) -> String {
        let rendered_type = (&arg.argument_type).into_renderer().render(ctx);

        format!("{}: {}", arg.name, rendered_type)
    }
}
