use super::*;

#[derive(Debug)]
pub(crate) enum DmmfObjectRenderer<'a> {
    Input(InputObjectType<'a>),
    Output(ObjectType<'a>),
}

impl<'a> Renderer<'a> for DmmfObjectRenderer<'a> {
    fn render(&self, ctx: &mut RenderContext<'a>) {
        match &self {
            DmmfObjectRenderer::Input(input_object) => match input_object.tag() {
                Some(ObjectTag::FieldRefType(_)) => self.render_field_ref_type(input_object, ctx),
                _ => self.render_input_object(input_object, ctx),
            },
            DmmfObjectRenderer::Output(output) => self.render_output_object(output, ctx),
        }
    }
}

impl<'a> DmmfObjectRenderer<'a> {
    fn render_input_object(&self, input_object: &InputObjectType<'a>, ctx: &mut RenderContext<'a>) {
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

        let meta = match (input_object.tag(), input_object.container()) {
            (None, None) => None,
            (tag, container) => Some(DmmfInputTypeMeta {
                source: tag.and_then(|tag| match tag {
                    ObjectTag::WhereInputType(c) => Some(c.name()),
                    _ => None,
                }),
                grouping: container.and_then(|c| Some(c.name())),
            }),
        };

        let input_type = DmmfInputType {
            name: input_object.identifier.name(),
            constraints: DmmfInputTypeConstraints {
                max_num_fields: input_object.constraints.max_num_fields,
                min_num_fields: input_object.constraints.min_num_fields,
                fields: input_object
                    .constraints
                    .fields
                    .as_ref()
                    .map(|f| f.iter().map(|s| s.clone().into_owned()).collect()),
            },
            fields: rendered_fields,
            meta,
        };

        ctx.add_input_type(input_object.identifier.clone(), input_type);
    }

    fn render_field_ref_type(&self, input_object: &InputObjectType<'a>, ctx: &mut RenderContext<'a>) {
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

        let allow_type = match input_object.tag() {
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

    fn render_output_object(&self, output_object: &ObjectType<'a>, ctx: &mut RenderContext<'a>) {
        if ctx.already_rendered(output_object.identifier()) {
            return;
        }

        // This will prevent the type and its fields to be re-rendered.
        ctx.mark_as_rendered(output_object.identifier().clone());

        let fields = output_object.get_fields();
        let mut rendered_fields: Vec<DmmfOutputField> = Vec::with_capacity(fields.len());

        for field in fields {
            rendered_fields.push(render_output_field(field, ctx))
        }

        let output_type = DmmfOutputType {
            name: output_object.name(),
            fields: rendered_fields,
        };

        ctx.add_output_type(output_object.identifier().clone(), output_type);
    }
}
