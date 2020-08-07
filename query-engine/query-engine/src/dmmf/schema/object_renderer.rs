use super::*;

#[derive(Debug)]
pub enum DMMFObjectRenderer {
    Input(InputObjectTypeWeakRef),
    Output(ObjectTypeWeakRef),
}

impl<'a> Renderer<'a, ()> for DMMFObjectRenderer {
    fn render(&self, ctx: &mut RenderContext) {
        match &self {
            DMMFObjectRenderer::Input(input) => self.render_input_object(input, ctx),
            DMMFObjectRenderer::Output(output) => self.render_output_object(output, ctx),
        }
    }
}

impl DMMFObjectRenderer {
    fn render_input_object(&self, input_object: &InputObjectTypeWeakRef, ctx: &mut RenderContext) {
        let input_object = input_object.into_arc();
        if ctx.already_rendered(&input_object.name) {
            return;
        } else {
            // This short circuits recursive processing for fields.
            ctx.mark_as_rendered(input_object.name.clone())
        }

        let fields = input_object.get_fields();
        let mut rendered_fields = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(render_input_field(&field, ctx));
        }

        let input_type = DMMFInputType {
            name: input_object.name.clone(),
            is_one_of: input_object.is_one_of,
            fields: rendered_fields,
        };

        ctx.add_input_type(input_type);
    }

    // WIP dedup code
    fn render_output_object(&self, output_object: &ObjectTypeWeakRef, ctx: &mut RenderContext) {
        let output_object = output_object.into_arc();
        if ctx.already_rendered(output_object.name()) {
            return;
        } else {
            // This short circuits recursive processing for fields.
            ctx.mark_as_rendered(output_object.name().to_string())
        }

        let fields = output_object.get_fields();
        let mut rendered_fields: Vec<DMMFField> = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(render_output_field(&field, ctx))
        }

        let output_type = DMMFOutputType {
            name: output_object.name().to_string(),
            fields: rendered_fields,
        };

        ctx.add_output_type(output_type);
    }
}
