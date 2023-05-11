use super::*;

#[derive(Debug)]
pub enum GqlObjectRenderer {
    Input(InputObjectTypeId),
    Output(OutputObjectTypeId),
}

impl Renderer for GqlObjectRenderer {
    fn render(&self, ctx: &mut RenderContext) -> String {
        match &self {
            GqlObjectRenderer::Input(input) => self.render_input_object(*input, ctx),
            GqlObjectRenderer::Output(output) => self.render_output_object(&ctx.query_schema.db[*output], ctx),
        }
    }
}

impl GqlObjectRenderer {
    fn render_input_object(&self, input_object: InputObjectTypeId, ctx: &mut RenderContext) -> String {
        let input_object = &ctx.query_schema.db[input_object];
        if ctx.already_rendered(&input_object.identifier.name()) {
            return "".into();
        } else {
            // This short circuits recursive processing for fields.
            ctx.mark_as_rendered(input_object.identifier.name())
        }

        let fields = input_object.get_fields();
        let mut rendered_fields = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(field.as_renderer().render(ctx))
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

        ctx.add(input_object.identifier.name(), rendered.clone());

        rendered
    }

    fn render_output_object(&self, output_object: &ObjectType, ctx: &mut RenderContext) -> String {
        if ctx.already_rendered(&output_object.identifier.name()) {
            return "".into();
        } else {
            // This short circuits recursive processing for fields.
            ctx.mark_as_rendered(output_object.identifier.name())
        }

        let fields = output_object.get_fields();
        let mut rendered_fields = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(field.as_renderer().render(ctx))
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
