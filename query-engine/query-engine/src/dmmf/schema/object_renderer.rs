use super::*;

#[derive(Debug)]
pub enum DmmfObjectRenderer {
    Input(InputObjectTypeWeakRef),
    Output(ObjectTypeWeakRef),
}

impl Renderer for DmmfObjectRenderer {
    fn render(&self, ctx: &mut RenderContext) {
        match &self {
            DmmfObjectRenderer::Input(input) => self.render_input_object(input, ctx),
            DmmfObjectRenderer::Output(output) => self.render_output_object(output, ctx),
        }
    }
}

impl DmmfObjectRenderer {
    fn render_input_object(&self, input_object: &InputObjectTypeWeakRef, ctx: &mut RenderContext) {
        let input_object = input_object.into_arc();

        if ctx.already_rendered(&input_object.name) {
            return;
        }

        // This will prevent the type and its fields to be re-rendered.
        ctx.mark_as_rendered(input_object.name.clone());

        let fields = input_object.get_fields();
        let mut rendered_fields = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(render_input_field(&field, ctx));
        }

        let input_type = DmmfInputType {
            name: input_object.name.clone(),
            constraints: DmmfInputTypeConstraints {
                max_num_fields: input_object.constraints.max_num_fields,
                min_num_fields: input_object.constraints.min_num_fields,
            },
            fields: rendered_fields,
        };

        ctx.add_input_type(input_type);
    }

    fn render_output_object(&self, output_object: &ObjectTypeWeakRef, ctx: &mut RenderContext) {
        let output_object = output_object.into_arc();

        if ctx.already_rendered(&output_object.name()) {
            return;
        }

        // This will prevent the type and its fields to be re-rendered.
        ctx.mark_as_rendered(output_object.name().to_owned());

        let fields = output_object.get_fields();
        let mut rendered_fields: Vec<DmmfOutputField> = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(render_output_field(&field, ctx))
        }

        let output_type = DmmfOutputType {
            name: output_object.name().to_string(),
            fields: rendered_fields,
        };

        ctx.add_output_type(output_type);
    }
}
