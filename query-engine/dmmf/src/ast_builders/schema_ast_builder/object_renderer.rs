use super::*;

#[derive(Debug)]
pub(crate) enum DmmfObjectRenderer {
    Input(InputObjectTypeId),
    Output(OutputObjectTypeId),
}

impl Renderer for DmmfObjectRenderer {
    fn render(&self, ctx: &mut RenderContext) {
        match &self {
            DmmfObjectRenderer::Input(input) => {
                let input_object = &ctx.query_schema.db[*input];

                match &input_object.tag {
                    Some(ObjectTag::FieldRefType(_)) => self.render_field_ref_type(input_object, ctx),
                    _ => self.render_input_object(input_object, ctx),
                }
            }
            DmmfObjectRenderer::Output(output) => self.render_output_object(&ctx.query_schema.db[*output], ctx),
        }
    }
}

impl DmmfObjectRenderer {
    fn render_input_object(&self, input_object: &InputObjectType, ctx: &mut RenderContext) {
        if ctx.already_rendered(&input_object.identifier) {
            return;
        }

        // This will prevent the type and its fields to be re-rendered.
        ctx.mark_as_rendered(input_object.identifier.clone());

        let fields = input_object.get_fields();
        let mut rendered_fields = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(render_input_field(field, ctx));
        }

        let meta = input_object.tag.as_ref().and_then(|tag| match tag {
            ObjectTag::WhereInputType(container) => Some(DmmfInputTypeMeta {
                source: Some(container.name()),
            }),
            _ => None,
        });

        let input_type = DmmfInputType {
            name: input_object.identifier.name(),
            constraints: DmmfInputTypeConstraints {
                max_num_fields: input_object.constraints.max_num_fields,
                min_num_fields: input_object.constraints.min_num_fields,
                fields: input_object.constraints.fields.as_ref().cloned(),
            },
            fields: rendered_fields,
            meta,
        };

        ctx.add_input_type(input_object.identifier.clone(), input_type);
    }

    fn render_field_ref_type(&self, input_object: &InputObjectType, ctx: &mut RenderContext) {
        if ctx.already_rendered(&input_object.identifier) {
            return;
        }

        // This will prevent the type and its fields to be re-rendered.
        ctx.mark_as_rendered(input_object.identifier.clone());

        let fields = input_object.get_fields();
        let mut rendered_fields = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(render_input_field(field, ctx));
        }

        let allow_type = match &input_object.tag {
            Some(ObjectTag::FieldRefType(input_type)) => input_type,
            _ => unreachable!(),
        };

        let field_ref_type = DmmfFieldRefType {
            name: input_object.identifier.name(),
            allow_types: vec![render_input_type(allow_type, ctx)],
            fields: rendered_fields,
        };

        ctx.add_field_ref_type(input_object.identifier.clone(), field_ref_type);
    }

    fn render_output_object(&self, output_object: &ObjectType, ctx: &mut RenderContext) {
        if ctx.already_rendered(&output_object.identifier) {
            return;
        }

        // This will prevent the type and its fields to be re-rendered.
        ctx.mark_as_rendered(output_object.identifier.clone());

        let fields = output_object.get_fields();
        let mut rendered_fields: Vec<DmmfOutputField> = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(render_output_field(field, ctx))
        }

        let output_type = DmmfOutputType {
            name: output_object.identifier.name(),
            fields: rendered_fields,
        };

        ctx.add_output_type(output_object.identifier.clone(), output_type);
    }
}
