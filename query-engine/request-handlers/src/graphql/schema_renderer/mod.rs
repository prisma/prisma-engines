mod enum_renderer;
mod field_renderer;
mod object_renderer;
mod type_renderer;

use enum_renderer::*;
use field_renderer::*;
use object_renderer::*;
use query_core::schema::*;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use type_renderer::*;

#[allow(dead_code)]
pub struct GraphQLSchemaRenderer;

/// Top level GraphQL schema renderer.
pub struct GqlSchemaRenderer {
    query_schema: QuerySchemaRef,
}

impl Renderer for GqlSchemaRenderer {
    fn render(&self, ctx: &mut RenderContext) -> String {
        let _ = self.query_schema.query.into_renderer().render(ctx);
        self.query_schema.mutation.into_renderer().render(ctx)
    }
}

impl GqlSchemaRenderer {
    pub fn new(query_schema: QuerySchemaRef) -> GqlSchemaRenderer {
        GqlSchemaRenderer { query_schema }
    }
}

impl QuerySchemaRenderer<String> for GraphQLSchemaRenderer {
    fn render(query_schema: QuerySchemaRef) -> String {
        let mut context = RenderContext::new();
        query_schema.into_renderer().render(&mut context);

        // Add custom scalar types (required for graphql.js implementations)
        format!(
            "{}\n\nscalar DateTime\nscalar Json\nscalar UUID\nscalar BigInt\nscalar Decimal",
            context.format()
        )
    }
}

pub trait Renderer {
    fn render(&self, ctx: &mut RenderContext) -> String;
}

pub struct RenderContext {
    /// Output queue for all (top level) elements that need to be rendered,
    output_queue: Vec<String>,

    /// Prevents double rendering of elements that are referenced multiple times.
    rendered: HashMap<String, ()>,

    /// General indent level in spaces.
    indent: usize,

    /// Indent string.
    indent_str: &'static str,
}

impl Default for RenderContext {
    fn default() -> Self {
        Self {
            output_queue: vec![],
            rendered: HashMap::new(),
            indent: 2,
            indent_str: " ",
        }
    }
}

impl RenderContext {
    pub fn new() -> RenderContext {
        Self::default()
    }

    pub fn format(self) -> String {
        self.output_queue.join("\n\n")
    }

    pub fn already_rendered(&self, cache_key: &str) -> bool {
        self.rendered.contains_key(cache_key)
    }

    pub fn mark_as_rendered(&mut self, cache_key: String) {
        self.rendered.insert(cache_key, ());
    }

    pub fn add_output(&mut self, output: String) {
        self.output_queue.push(output);
    }

    pub fn add(&mut self, cache_key: String, output: String) {
        self.add_output(output);
        self.mark_as_rendered(cache_key);
    }

    pub fn indent(&self) -> String {
        self.indent_str.repeat(self.indent)
    }
}

enum GqlRenderer<'a> {
    Schema(GqlSchemaRenderer),
    Object(GqlObjectRenderer),
    Type(GqlTypeRenderer<'a>),
    Field(GqlFieldRenderer),
    Enum(GqlEnumRenderer<'a>),
}

impl<'a> Renderer for GqlRenderer<'a> {
    fn render(&self, ctx: &mut RenderContext) -> String {
        match self {
            GqlRenderer::Schema(s) => s.render(ctx),
            GqlRenderer::Object(o) => o.render(ctx),
            GqlRenderer::Type(t) => t.render(ctx),
            GqlRenderer::Field(f) => f.render(ctx),
            GqlRenderer::Enum(e) => e.render(ctx),
        }
    }
}

trait IntoRenderer<'a> {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&'a self) -> GqlRenderer<'a>;
}

impl<'a> IntoRenderer<'a> for QuerySchemaRef {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Schema(GqlSchemaRenderer::new(Arc::clone(self)))
    }
}

impl<'a> IntoRenderer<'a> for &'a InputType {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Type(GqlTypeRenderer::Input(self))
    }
}

impl<'a> IntoRenderer<'a> for OutputType {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&'a self) -> GqlRenderer<'a> {
        GqlRenderer::Type(GqlTypeRenderer::Output(self))
    }
}

impl<'a> IntoRenderer<'a> for InputFieldRef {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Field(GqlFieldRenderer::Input(Arc::clone(self)))
    }
}

impl<'a> IntoRenderer<'a> for OutputFieldRef {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Field(GqlFieldRenderer::Output(Arc::clone(self)))
    }
}

impl<'a> IntoRenderer<'a> for EnumType {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&'a self) -> GqlRenderer<'a> {
        GqlRenderer::Enum(GqlEnumRenderer::new(self))
    }
}

impl<'a> IntoRenderer<'a> for &'a InputObjectTypeWeakRef {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Object(GqlObjectRenderer::Input(Weak::clone(self)))
    }
}

impl<'a> IntoRenderer<'a> for &'a ObjectTypeWeakRef {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Object(GqlObjectRenderer::Output(Weak::clone(self)))
    }
}
