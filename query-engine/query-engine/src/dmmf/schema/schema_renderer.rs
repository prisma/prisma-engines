use super::*;

pub struct DMMFSchemaRenderer {
    query_schema: QuerySchemaRef,
}

impl<'a> Renderer<'a> for DMMFSchemaRenderer {
    fn render(&self, ctx: &mut RenderContext) {
        render_output_type(&self.query_schema.query, ctx);
        render_output_type(&self.query_schema.mutation, ctx);
    }
}

impl DMMFSchemaRenderer {
    pub fn new(query_schema: QuerySchemaRef) -> DMMFSchemaRenderer {
        DMMFSchemaRenderer { query_schema }
    }
}
