use super::*;

#[derive(Debug)]
pub enum DMMFFieldRenderer {
    Input(InputFieldRef),
    Output(FieldRef),
}

impl<'a> Renderer<'a, DMMFFieldWrapper> for DMMFFieldRenderer {
    fn render(&self, ctx: &mut RenderContext) -> DMMFFieldWrapper {
        match self {
            DMMFFieldRenderer::Input(input) => self.render_input_field(Arc::clone(input), ctx),
            DMMFFieldRenderer::Output(output) => self.render_output_field(Arc::clone(output), ctx),
        }
    }
}

impl DMMFFieldRenderer {
    fn render_input_field(&self, input_field: InputFieldRef, ctx: &mut RenderContext) -> DMMFFieldWrapper {
        let type_info = input_field.field_type.into_renderer().render(ctx);
        let field = DMMFInputField {
            name: input_field.name.clone(),
            input_type: type_info,
        };

        DMMFFieldWrapper::Input(field)
    }

    fn render_output_field(&self, field: FieldRef, ctx: &mut RenderContext) -> DMMFFieldWrapper {
        let args = self.render_arguments(&field.arguments, ctx);
        let output_type = field.field_type.into_renderer().render(ctx);
        let output_field = DMMFField {
            name: field.name.clone(),
            args,
            output_type,
        };

        ctx.add_mapping(field.name.clone(), field.query_builder.as_ref());
        DMMFFieldWrapper::Output(output_field)
    }

    fn render_arguments(&self, args: &[Argument], ctx: &mut RenderContext) -> Vec<DMMFArgument> {
        args.iter().map(|arg| self.render_argument(arg, ctx)).collect()
    }

    fn render_argument(&self, arg: &Argument, ctx: &mut RenderContext) -> DMMFArgument {
        let input_type = (&arg.argument_type).into_renderer().render(ctx);
        let rendered_arg = DMMFArgument {
            name: arg.name.clone(),
            input_type,
        };

        rendered_arg
    }
}
