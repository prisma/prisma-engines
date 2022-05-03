use super::*;

pub struct DmmfSchemaRenderer {
    query_schema: QuerySchemaRef,
}

impl Renderer for DmmfSchemaRenderer {
    fn render(&self, ctx: &mut RenderContext) {
        render_output_type(&self.query_schema.query, ctx);
        render_output_type(&self.query_schema.mutation, ctx);
    }
}

impl DmmfSchemaRenderer {
    pub fn new(query_schema: QuerySchemaRef) -> DmmfSchemaRenderer {
        DmmfSchemaRenderer { query_schema }
    }
}
