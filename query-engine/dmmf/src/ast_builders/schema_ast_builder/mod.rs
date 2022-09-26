mod enum_renderer;
mod field_renderer;
mod object_renderer;
mod schema_renderer;
mod type_renderer;

use crate::serialization_ast::{mappings_ast::*, schema_ast::*};
use enum_renderer::*;
use field_renderer::*;
use indexmap::map::Entry;
use object_renderer::*;
use schema::*;
use schema_renderer::*;
use std::{
    collections::HashSet,
    sync::{Arc, Weak},
};
use type_renderer::*;

pub struct DmmfQuerySchemaRenderer;

impl QuerySchemaRenderer<(DmmfSchema, DmmfOperationMappings)> for DmmfQuerySchemaRenderer {
    fn render(query_schema: QuerySchemaRef) -> (DmmfSchema, DmmfOperationMappings) {
        let mut ctx = RenderContext::new();
        ctx.mark_to_be_rendered(&query_schema);

        while !ctx.next_pass.is_empty() {
            let renderers = std::mem::take(&mut ctx.next_pass);

            for renderer in renderers {
                renderer.render(&mut ctx)
            }
        }

        ctx.finalize()
    }
}

#[derive(Default)]
pub struct RenderContext {
    /// Aggregator for query schema
    schema: DmmfSchema,

    /// Aggregator for operation mappings
    mappings: DmmfOperationMappings,

    /// Prevents double rendering of elements that are referenced multiple times.
    /// Names of input / output types / enums / models are unique by namespace.
    rendered: HashSet<Identifier>,

    /// The child objects to render next. Rendering is considered complete when
    /// this is empty.
    next_pass: Vec<Box<dyn Renderer>>,
}

impl RenderContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn finalize(self) -> (DmmfSchema, DmmfOperationMappings) {
        (self.schema, self.mappings)
    }

    pub fn already_rendered(&self, cache_key: &Identifier) -> bool {
        self.rendered.contains(cache_key)
    }

    pub fn mark_as_rendered(&mut self, cache_key: Identifier) {
        self.rendered.insert(cache_key);
    }

    pub fn add_enum(&mut self, identifier: Identifier, dmmf_enum: DmmfEnum) {
        // Enums from the namespace
        match self.schema.enum_types.entry(identifier.namespace().to_owned()) {
            Entry::Occupied(mut v) => v.get_mut().push(dmmf_enum),
            Entry::Vacant(v) => {
                v.insert(vec![dmmf_enum]);
            }
        };

        self.mark_as_rendered(identifier);
    }

    pub fn add_input_type(&mut self, identifier: Identifier, input_type: DmmfInputType) {
        // Input types from the namespace
        match self.schema.input_object_types.entry(identifier.namespace().to_owned()) {
            Entry::Occupied(mut v) => v.get_mut().push(input_type),
            Entry::Vacant(v) => {
                v.insert(vec![input_type]);
            }
        };

        self.mark_as_rendered(identifier);
    }

    pub fn add_output_type(&mut self, identifier: Identifier, output_type: DmmfOutputType) {
        // Output types from the namespace
        match self.schema.output_object_types.entry(identifier.namespace().to_owned()) {
            Entry::Occupied(mut v) => v.get_mut().push(output_type),
            Entry::Vacant(v) => {
                v.insert(vec![output_type]);
            }
        };

        self.mark_as_rendered(identifier);
    }

    pub fn add_field_ref_type(&mut self, identifier: Identifier, ref_type: DmmfFieldRefType) {
        // Field ref types from the namespace
        match self.schema.field_ref_types.entry(identifier.namespace().to_owned()) {
            Entry::Occupied(mut v) => v.get_mut().push(ref_type),
            Entry::Vacant(v) => {
                v.insert(vec![ref_type]);
            }
        };

        self.mark_as_rendered(identifier);
    }

    pub fn add_mapping(&mut self, name: String, operation: Option<&QueryInfo>) {
        if let Some(info) = operation {
            if let Some(ref model) = info.model {
                let model_name = model.name.clone();
                let tag_str = match &info.tag {
                    // If it's a QueryRaw with a query_type, then use this query_type as operation name
                    // eg: findRaw, aggregateRaw..
                    QueryTag::QueryRaw { query_type } if query_type.is_some() => {
                        query_type.as_ref().unwrap().to_owned()
                    }
                    tag => format!("{}", tag),
                };
                let model_op = self
                    .mappings
                    .model_operations
                    .iter()
                    .find(|mapping| mapping.model_name == model_name);

                match model_op {
                    Some(existing) => existing.add_operation(tag_str, name),
                    None => {
                        let new_mapping = DmmfModelOperations::new(model_name);

                        new_mapping.add_operation(tag_str, name);
                        self.mappings.model_operations.push(new_mapping);
                    }
                };
            } else {
                match &info.tag {
                    // If it's a QueryRaw with a query_type, then use this query_type as operation name
                    // eg: runCommandRaw
                    QueryTag::QueryRaw { query_type } if query_type.is_some() => {
                        self.mappings
                            .other_operations
                            .write
                            .push(query_type.as_ref().unwrap().to_owned());
                    }
                    QueryTag::ExecuteRaw | QueryTag::QueryRaw { query_type: _ } => {
                        self.mappings.other_operations.write.push(info.tag.to_string());
                    }
                    _ => unreachable!("Invalid operations mapping."),
                }
            }
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
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&self) -> Box<dyn Renderer>;

    /// Returns whether the item still needs to be rendered.
    fn is_already_rendered(&self, ctx: &RenderContext) -> bool;
}

impl IntoRenderer for QuerySchemaRef {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&self) -> Box<dyn Renderer> {
        Box::new(DmmfSchemaRenderer::new(Arc::clone(self)))
    }

    fn is_already_rendered(&self, _ctx: &RenderContext) -> bool {
        false
    }
}

impl<'a> IntoRenderer for &'a EnumType {
    #[allow(clippy::wrong_self_convention)]
    fn into_renderer(&self) -> Box<dyn Renderer> {
        Box::new(DmmfEnumRenderer::new(self))
    }

    fn is_already_rendered(&self, ctx: &RenderContext) -> bool {
        ctx.already_rendered(self.identifier())
    }
}

impl IntoRenderer for InputObjectTypeWeakRef {
    fn into_renderer(&self) -> Box<dyn Renderer> {
        Box::new(DmmfObjectRenderer::Input(Weak::clone(self)))
    }

    fn is_already_rendered(&self, ctx: &RenderContext) -> bool {
        ctx.already_rendered(&self.into_arc().identifier)
    }
}

impl IntoRenderer for ObjectTypeWeakRef {
    fn into_renderer(&self) -> Box<dyn Renderer> {
        Box::new(DmmfObjectRenderer::Output(Weak::clone(self)))
    }

    fn is_already_rendered(&self, ctx: &RenderContext) -> bool {
        ctx.already_rendered(&self.into_arc().identifier)
    }
}
