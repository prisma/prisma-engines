mod enum_renderer;
mod field_renderer;
mod object_renderer;
mod type_renderer;

use enum_renderer::*;
use field_renderer::*;
use object_renderer::*;
use query_core::schema::*;
use std::collections::HashMap;
use type_renderer::*;

/// Top level GraphQL schema renderer.
struct GqlSchemaRenderer<'a> {
    query_schema: &'a QuerySchema,
}

impl<'a> Renderer for GqlSchemaRenderer<'a> {
    fn render(&self, ctx: &mut RenderContext) -> String {
        let _ = self.query_schema.query().as_renderer().render(ctx);
        self.query_schema.mutation().as_renderer().render(ctx)
    }
}

impl<'a> GqlSchemaRenderer<'a> {
    fn new(query_schema: &'a QuerySchema) -> GqlSchemaRenderer<'a> {
        GqlSchemaRenderer { query_schema }
    }
}

pub fn render_graphql_schema(query_schema: &QuerySchema) -> String {
    let mut context = RenderContext::new();
    query_schema.as_renderer().render(&mut context);

    // Add custom scalar types (required for graphql.js implementations)
    format!(
        "{}\n\nscalar DateTime\nscalar Json\nscalar UUID\nscalar BigInt\nscalar Decimal\nscalar Bytes",
        context.format()
    )
}

trait Renderer {
    fn render(&self, ctx: &mut RenderContext) -> String;
}

struct RenderContext {
    /// Output queue for all (top level) elements that need to be rendered,
    output_queue: Vec<String>,

    /// Prevents double rendering of elements that are referenced multiple times.
    rendered: HashMap<String, ()>,

    /// General indent level in spaces.
    indent: usize,

    /// Indent string.
    indent_str: &'static str,
}

impl RenderContext {
    fn new() -> Self {
        RenderContext {
            output_queue: Default::default(),
            rendered: Default::default(),
            indent: 2,
            indent_str: " ",
        }
    }

    fn format(self) -> String {
        self.output_queue.join("\n\n")
    }

    fn already_rendered(&self, cache_key: &str) -> bool {
        self.rendered.contains_key(cache_key)
    }

    fn mark_as_rendered(&mut self, cache_key: String) {
        self.rendered.insert(cache_key, ());
    }

    fn add_output(&mut self, output: String) {
        self.output_queue.push(output);
    }

    fn add(&mut self, cache_key: String, output: String) {
        self.add_output(output);
        self.mark_as_rendered(cache_key);
    }

    fn indent(&self) -> String {
        self.indent_str.repeat(self.indent)
    }
}

enum GqlRenderer<'a> {
    Schema(GqlSchemaRenderer<'a>),
    Object(GqlObjectRenderer<'a>),
    Type(GqlTypeRenderer<'a>),
    Field(GqlFieldRenderer<'a>),
    Enum(GqlEnumRenderer),
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

trait AsRenderer<'a> {
    fn as_renderer(&self) -> GqlRenderer<'a>;
}

impl<'a> AsRenderer<'a> for &'a QuerySchema {
    fn as_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Schema(GqlSchemaRenderer::new(self))
    }
}

impl<'a> AsRenderer<'a> for InputType<'a> {
    fn as_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Type(GqlTypeRenderer::Input(self.clone()))
    }
}

impl<'a> AsRenderer<'a> for OutputType<'a> {
    fn as_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Type(GqlTypeRenderer::Output(self.clone()))
    }
}

impl<'a> AsRenderer<'a> for InputField<'a> {
    fn as_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Field(GqlFieldRenderer::Input(self.clone()))
    }
}

impl<'a> AsRenderer<'a> for OutputField<'a> {
    fn as_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Field(GqlFieldRenderer::Output(self.clone()))
    }
}

impl<'a> AsRenderer<'a> for EnumType {
    fn as_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Enum(GqlEnumRenderer::new(self.clone()))
    }
}

impl<'a> AsRenderer<'a> for InputObjectType<'a> {
    fn as_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Object(GqlObjectRenderer::Input(self.clone()))
    }
}

impl<'a> AsRenderer<'a> for ObjectType<'a> {
    fn as_renderer(&self) -> GqlRenderer<'a> {
        GqlRenderer::Object(GqlObjectRenderer::Output(self.clone()))
    }
}
