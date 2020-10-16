mod ast;
mod enum_renderer;

mod field_renderer;
mod object_renderer;
mod schema_renderer;
mod type_renderer;

use crate::dmmf::DmmfMapping;
use enum_renderer::*;
use field_renderer::*;
use object_renderer::*;
use query_core::schema::*;
use schema_renderer::*;
use std::{
    collections::HashSet,
    sync::{Arc, Weak},
};
use type_renderer::*;

pub use ast::*;

pub struct DmmfQuerySchemaRenderer;

impl QuerySchemaRenderer<(DmmfSchema, Vec<DmmfMapping>)> for DmmfQuerySchemaRenderer {
    fn render(query_schema: QuerySchemaRef) -> (DmmfSchema, Vec<DmmfMapping>) {
        let mut ctx = RenderContext::new();
        ctx.mark_to_be_rendered(&query_schema);

        while !ctx.next_pass.is_empty() {
            let renderers = std::mem::replace(&mut ctx.next_pass, Vec::new());

            for renderer in renderers {
                renderer.render(&mut ctx)
            }
        }

        ctx.finalize()
    }
}

pub struct RenderContext {
    /// Aggregator for query schema
    schema: DmmfSchema,

    /// Aggregator for mappings
    mappings: Vec<DmmfMapping>,

    /// Prevents double rendering of elements that are referenced multiple times.
    /// Names of input / output types / enums / models are globally unique.
    rendered: HashSet<String>,

    /// The child objects to render next. Rendering is considered complete when
    /// this is empty.
    next_pass: Vec<Box<dyn Renderer>>,
}

impl RenderContext {
    pub fn new() -> Self {
        RenderContext {
            schema: DmmfSchema::default(),
            mappings: vec![],
            rendered: HashSet::new(),
            next_pass: Vec::new(),
        }
    }

    pub fn finalize(self) -> (DmmfSchema, Vec<DmmfMapping>) {
        let mut schema = self.schema;

        schema.root_query_type = "Query".into();
        schema.root_mutation_type = "Mutation".into();

        (schema, self.mappings)
    }

    pub fn already_rendered(&self, cache_key: &str) -> bool {
        self.rendered.contains(cache_key)
    }

    pub fn mark_as_rendered(&mut self, cache_key: String) {
        self.rendered.insert(cache_key);
    }

    pub fn add_enum(&mut self, name: String, dmmf_enum: DmmfEnum) {
        self.schema.enums.push(dmmf_enum);
        self.mark_as_rendered(name);
    }

    pub fn add_input_type(&mut self, input_type: DmmfInputType) {
        self.mark_as_rendered(input_type.name.clone());
        self.schema.input_types.push(input_type);
    }

    pub fn add_output_type(&mut self, output_type: DmmfOutputType) {
        self.mark_as_rendered(output_type.name.clone());
        self.schema.output_types.push(output_type);
    }

    pub fn add_mapping(&mut self, name: String, operation: Option<&SchemaQueryBuilder>) {
        if let Some(SchemaQueryBuilder::ModelQueryBuilder(m)) = operation {
            let model_name = m.model.name.clone();
            let tag_str = format!("{}", m.tag);
            let mapping = self.mappings.iter().find(|mapping| mapping.model_name == model_name);

            match mapping {
                Some(ref existing) => existing.add_operation(tag_str, name),
                None => {
                    let new_mapping = DmmfMapping::new(model_name);

                    new_mapping.add_operation(tag_str, name);
                    self.mappings.push(new_mapping);
                }
            };
        }
    }

    fn mark_to_be_rendered(&mut self, into_renderer: &dyn IntoRenderer) {
        if !into_renderer.is_already_rendered(self) {
            let renderer: Box<dyn Renderer> = into_renderer.into_renderer();
            self.next_pass.push(renderer)
        }
    }
}

pub trait Renderer {
    fn render(&self, ctx: &mut RenderContext);
}

trait IntoRenderer {
    fn into_renderer(&self) -> Box<dyn Renderer>;

    /// Returns whether the item still needs to be rendered.
    fn is_already_rendered(&self, ctx: &RenderContext) -> bool;
}

impl IntoRenderer for QuerySchemaRef {
    fn into_renderer(&self) -> Box<dyn Renderer> {
        Box::new(DmmfSchemaRenderer::new(Arc::clone(self)))
    }

    fn is_already_rendered(&self, _ctx: &RenderContext) -> bool {
        false
    }
}

impl<'a> IntoRenderer for &'a EnumType {
    fn into_renderer(&self) -> Box<dyn Renderer> {
        Box::new(DmmfEnumRenderer::new(self))
    }

    fn is_already_rendered(&self, ctx: &RenderContext) -> bool {
        ctx.already_rendered(self.name())
    }
}

impl IntoRenderer for InputObjectTypeWeakRef {
    fn into_renderer(&self) -> Box<dyn Renderer> {
        Box::new(DmmfObjectRenderer::Input(Weak::clone(self)))
    }

    fn is_already_rendered(&self, ctx: &RenderContext) -> bool {
        ctx.already_rendered(&self.into_arc().name)
    }
}

impl IntoRenderer for ObjectTypeWeakRef {
    fn into_renderer(&self) -> Box<dyn Renderer> {
        Box::new(DmmfObjectRenderer::Output(Weak::clone(self)))
    }

    fn is_already_rendered(&self, ctx: &RenderContext) -> bool {
        ctx.already_rendered(self.into_arc().name())
    }
}
