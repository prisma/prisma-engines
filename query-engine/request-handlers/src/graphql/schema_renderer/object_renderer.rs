use super::*;

#[derive(Debug)]
pub enum GqlObjectRenderer {
    Input(InputObjectTypeWeakRef),
    Output(ObjectTypeWeakRef),
}

impl Renderer for GqlObjectRenderer {
    fn render(&self, ctx: &mut RenderContext) -> String {
        match &self {
            GqlObjectRenderer::Input(input) => self.render_input_object(input, ctx),
            GqlObjectRenderer::Output(output) => self.render_output_object(output, ctx),
        }
    }
}

impl GqlObjectRenderer {
    fn render_input_object(&self, input_object: &InputObjectTypeWeakRef, ctx: &mut RenderContext) -> String {
        let input_object = input_object.into_arc();
        if ctx.already_rendered(input_object.identifier.name()) {
            return "".into();
        } else {
            // This short circuits recursive processing for fields.
            ctx.mark_as_rendered(input_object.identifier.name().to_owned())
        }

        let fields = input_object.get_fields();
        let mut rendered_fields = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(field.into_renderer().render(ctx))
        }

        let indented: Vec<String> = rendered_fields
            .into_iter()
            .map(|f| format!("{}{}", ctx.indent(), f))
            .collect();

        let rendered = format!(
            "input {} {{\n{}\n}}",
            input_object.identifier.name(),
            indented.join("\n")
        );

        ctx.add(input_object.identifier.name().to_owned(), rendered.clone());

        rendered
    }

    fn render_output_object(&self, output_object: &ObjectTypeWeakRef, ctx: &mut RenderContext) -> String {
        let output_object = output_object.into_arc();

        if ctx.already_rendered(output_object.identifier.name()) {
            return "".into();
        } else {
            // This short circuits recursive processing for fields.
            ctx.mark_as_rendered(output_object.identifier.name().to_string())
        }

        let fields = output_object.get_fields();
        let mut rendered_fields = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(field.into_renderer().render(ctx))
        }

        let indented: Vec<String> = rendered_fields
            .into_iter()
            .map(|f| format!("{}{}", ctx.indent(), f))
            .collect();

        let rendered = format!(
            "type {} {{\n{}\n}}",
            output_object.identifier.name(),
            indented.join("\n")
        );

        ctx.add_output(rendered.clone());

        rendered
    }
}
