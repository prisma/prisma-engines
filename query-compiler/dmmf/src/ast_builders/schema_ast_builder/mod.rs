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
use std::collections::HashSet;
use type_renderer::*;

pub(crate) fn render(query_schema: &QuerySchema) -> (DmmfSchema, DmmfOperationMappings) {
    let mut ctx = RenderContext::new(query_schema);
    ctx.mark_to_be_rendered(&query_schema);

    while !ctx.next_pass.is_empty() {
        let renderers = std::mem::take(&mut ctx.next_pass);

        for renderer in renderers {
            renderer.render(&mut ctx)
        }
    }

    ctx.finalize()
}

pub(crate) struct RenderContext<'a> {
    query_schema: &'a QuerySchema,

    /// Aggregator for query schema
    schema: DmmfSchema,

    /// Aggregator for operation mappings
    mappings: DmmfOperationMappings,

    /// Prevents double rendering of elements that are referenced multiple times.
    /// Names of input / output types / enums / models are unique by namespace.
    rendered: HashSet<Identifier>,

    /// The child objects to render next. Rendering is considered complete when
    /// this is empty.
    next_pass: Vec<Box<dyn Renderer<'a> + 'a>>,
}

impl<'a> RenderContext<'a> {
    fn new(query_schema: &'a QuerySchema) -> Self {
        RenderContext {
            query_schema,
            schema: Default::default(),
            mappings: Default::default(),
            rendered: Default::default(),
            next_pass: Default::default(),
        }
    }

    fn finalize(self) -> (DmmfSchema, DmmfOperationMappings) {
        (self.schema, self.mappings)
    }

    fn already_rendered(&self, cache_key: &Identifier) -> bool {
        self.rendered.contains(cache_key)
    }

    fn mark_as_rendered(&mut self, cache_key: Identifier) {
        self.rendered.insert(cache_key);
    }

    fn add_enum(&mut self, identifier: Identifier, dmmf_enum: DmmfEnum) {
        // Enums from the namespace
        match self.schema.enum_types.entry(identifier.namespace().to_owned()) {
            Entry::Occupied(mut v) => v.get_mut().push(dmmf_enum),
            Entry::Vacant(v) => {
                v.insert(vec![dmmf_enum]);
            }
        };

        self.mark_as_rendered(identifier);
    }

    fn add_input_type(&mut self, identifier: Identifier, input_type: DmmfInputType) {
        // Input types from the namespace
        match self.schema.input_object_types.entry(identifier.namespace().to_owned()) {
            Entry::Occupied(mut v) => v.get_mut().push(input_type),
            Entry::Vacant(v) => {
                v.insert(vec![input_type]);
            }
        };

        self.mark_as_rendered(identifier);
    }

    fn add_output_type(&mut self, identifier: Identifier, output_type: DmmfOutputType) {
        // Output types from the namespace
        match self.schema.output_object_types.entry(identifier.namespace().to_owned()) {
            Entry::Occupied(mut v) => v.get_mut().push(output_type),
            Entry::Vacant(v) => {
                v.insert(vec![output_type]);
            }
        };

        self.mark_as_rendered(identifier);
    }

    fn add_field_ref_type(&mut self, identifier: Identifier, ref_type: DmmfFieldRefType) {
        // Field ref types from the namespace
        match self.schema.field_ref_types.entry(identifier.namespace().to_owned()) {
            Entry::Occupied(mut v) => v.get_mut().push(ref_type),
            Entry::Vacant(v) => {
                v.insert(vec![ref_type]);
            }
        };

        self.mark_as_rendered(identifier);
    }

    pub(crate) fn add_mapping(&mut self, name: String, operation: Option<&QueryInfo>) {
        if let Some(info) = operation {
            if let Some(model) = info.model {
                let model = self.query_schema.internal_data_model.walk(model);
                let model_name = model.name();
                let tag_str = info.tag.to_string();
                let model_op = self
                    .mappings
                    .model_operations
                    .iter()
                    .find(|mapping| mapping.model_name == model_name);

                match model_op {
                    Some(existing) => existing.add_operation(tag_str, name),
                    None => {
                        let new_mapping = DmmfModelOperations::new(model_name.to_owned());

                        new_mapping.add_operation(tag_str, name);
                        self.mappings.model_operations.push(new_mapping);
                    }
                };
            } else {
                match &info.tag {
                    QueryTag::ExecuteRaw | QueryTag::QueryRaw | QueryTag::RunCommandRaw => {
                        self.mappings.other_operations.write.push(info.tag.to_string());
                    }
                    _ => unreachable!("Invalid operations mapping."),
                }
            }
        }
    }

    fn mark_to_be_rendered(&mut self, into_renderer: &(impl AsRenderer<'a> + 'a)) {
        if !into_renderer.is_already_rendered(self) {
            let renderer: Box<dyn Renderer> = into_renderer.as_renderer();
            self.next_pass.push(renderer)
        }
    }
}

pub(crate) trait Renderer<'a> {
    fn render(&self, ctx: &mut RenderContext<'a>);
}

trait AsRenderer<'a> {
    fn as_renderer(&self) -> Box<dyn Renderer<'a> + 'a>;

    /// Returns whether the item still needs to be rendered.
    fn is_already_rendered(&self, ctx: &RenderContext<'_>) -> bool;
}

impl<'a> AsRenderer<'a> for &'a QuerySchema {
    fn as_renderer(&self) -> Box<dyn Renderer<'a> + 'a> {
        Box::new(DmmfSchemaRenderer::new(self))
    }

    fn is_already_rendered(&self, _ctx: &RenderContext) -> bool {
        false
    }
}

impl<'a> AsRenderer<'a> for EnumType {
    fn as_renderer(&self) -> Box<dyn Renderer<'a> + 'a> {
        Box::new(DmmfEnumRenderer::new(self.clone()))
    }

    fn is_already_rendered(&self, ctx: &RenderContext) -> bool {
        ctx.already_rendered(self.identifier())
    }
}

impl<'a> AsRenderer<'a> for InputObjectType<'a> {
    fn as_renderer(&self) -> Box<dyn Renderer<'a> + 'a> {
        Box::new(DmmfObjectRenderer::Input(self.clone()))
    }

    fn is_already_rendered(&self, ctx: &RenderContext) -> bool {
        ctx.already_rendered(&self.identifier)
    }
}

impl<'a> AsRenderer<'a> for ObjectType<'a> {
    fn as_renderer(&self) -> Box<dyn Renderer<'a> + 'a> {
        Box::new(DmmfObjectRenderer::Output(self.clone()))
    }

    fn is_already_rendered(&self, ctx: &RenderContext) -> bool {
        ctx.already_rendered(self.identifier())
    }
}
