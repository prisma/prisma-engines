use super::*;

#[derive(Debug)]
pub(crate) enum GqlFieldRenderer<'a> {
    Input(InputField<'a>),
    Output(OutputField<'a>),
}

impl<'a> Renderer for GqlFieldRenderer<'a> {
    fn render(&self, ctx: &mut RenderContext) -> String {
        match self {
            GqlFieldRenderer::Input(input) => self.render_input_field(input, ctx),
            GqlFieldRenderer::Output(output) => self.render_output_field(output, ctx),
        }
    }
}

impl<'a> GqlFieldRenderer<'a> {
    fn render_input_field(&self, input_field: &InputField<'a>, ctx: &mut RenderContext) -> String {
        let rendered_type = pick_input_type(input_field.field_types()).as_renderer().render(ctx);
        let required = if input_field.is_required() { "!" } else { "" };

        format!("{}: {}{}", input_field.name, rendered_type, required)
    }

    fn render_output_field(&self, field: &OutputField<'a>, ctx: &mut RenderContext) -> String {
        let rendered_args = self.render_arguments(field.arguments().iter(), ctx);
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

        let rendered_type = field.field_type().as_renderer().render(ctx);
        let bang = if !field.is_nullable { "!" } else { "" };
        format!("{}{}: {}{}", field.name(), rendered_args, rendered_type, bang)
    }

    fn render_arguments<'b>(
        &'b self,
        args: impl Iterator<Item = &'b InputField<'a>>,
        ctx: &mut RenderContext,
    ) -> Vec<String> {
        let mut output = Vec::new();

        for arg in args {
            output.push(self.render_argument(arg, ctx))
        }

        output
    }

    fn render_argument(&self, arg: &InputField<'a>, ctx: &mut RenderContext) -> String {
        let rendered_type = pick_input_type(arg.field_types()).as_renderer().render(ctx);
        let required = if arg.is_required() { "!" } else { "" };

        format!("{}: {}{}", arg.name, rendered_type, required)
    }
}

/// GQL can't represent unions, so we pick the first object type.
/// If none is available, pick the first non-null scalar type.
///
/// Important: This doesn't really affect the functionality of the QE,
///            it's only serving the playground used for ad-hoc debugging.
fn pick_input_type<'a, 'b>(candidates: &'b [InputType<'a>]) -> &'b InputType<'a> {
    candidates
        .iter()
        .reduce(|prev, next| match (prev, next) {
            (InputType::Scalar(ScalarType::Null), _) => next, // Null has the least precedence.
            (InputType::Scalar(_), InputType::List(_)) => next, // Lists have precedence over scalars.
            (InputType::Scalar(_), InputType::Object(_)) => next, // Objects have precedence over scalars.
            _ => prev,
        })
        .expect("Expected at least one input type to be present.")
}
