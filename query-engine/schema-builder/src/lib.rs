//! Query schema builder. Root for query schema building.
//!
//! The schema builder creates all builders necessary for the process,
//! and hands down references to the individual initializers as required.
//!
//! Circular dependency schema building requires special consideration.
//! Assume a data model looks like this, with arrows indicating some kind of relation between models:
//!
//! ```text
//!       +---+
//!   +---+ B +<---+
//!   |   +---+    |
//!   v            |
//! +-+-+        +-+-+      +---+
//! | A +------->+ C +<-----+ D |
//! +---+        +---+      +---+
//! ```
//!
//! The above would cause infinite builder recursion circular
//! dependency (A -> B -> C -> A) in relations (for example in filter building).
//!
//! Without caching, processing D (in fact, visiting any type after the intial computation) would also
//! trigger a complete recomputation of A, B, C.
//!
//! Hence, all builders that produce input or output object types are required to
//! implement CachedBuilder in some form to break recursive type building.
//!
//! Additionally, the cache also acts as the component to prevent memory leaks from circular dependencies
//! in the query schema later on, as described on the QuerySchema type.
//! The cache can be consumed to produce a list of strong references to the individual input and output
//! object types, which are then moved to the query schema to keep weak references alive (see TypeRefCache for additional infos).

#[macro_use]
mod cache;
pub mod constants;
mod input_types;
mod output_types;
mod utils;

use cache::TypeRefCache;
use datamodel_connector::{ConnectorCapabilities, ConnectorCapability, ReferentialIntegrity};
use prisma_models::{
    datamodel::common::preview_features::PreviewFeature, CompositeTypeRef, Field as ModelField, Index,
    InternalDataModelRef, ModelRef, RelationFieldRef, TypeIdentifier,
};
use schema::*;
use std::sync::Arc;

pub use utils::*;

pub(crate) struct BuilderContext {
    internal_data_model: InternalDataModelRef,
    enable_raw_queries: bool,
    cache: TypeCache,
    capabilities: ConnectorCapabilities,
    preview_features: Vec<PreviewFeature>,
    nested_create_inputs_queue: NestedInputsQueue,
    nested_update_inputs_queue: NestedInputsQueue,
}

impl BuilderContext {
    pub fn new(
        internal_data_model: InternalDataModelRef,
        enable_raw_queries: bool,
        capabilities: ConnectorCapabilities,
        preview_features: Vec<PreviewFeature>,
    ) -> Self {
        Self {
            internal_data_model,
            enable_raw_queries,
            cache: TypeCache::new(),
            capabilities,
            preview_features,
            nested_create_inputs_queue: Vec::new(),
            nested_update_inputs_queue: Vec::new(),
        }
    }

    pub fn has_feature(&self, feature: &PreviewFeature) -> bool {
        self.preview_features.contains(feature)
    }

    pub fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get an input (object) type.
    pub fn get_input_type(&mut self, ident: &Identifier) -> Option<InputObjectTypeWeakRef> {
        self.cache.input_types.get(ident)
    }

    /// Get an output (object) type.
    pub fn get_output_type(&mut self, ident: &Identifier) -> Option<ObjectTypeWeakRef> {
        self.cache.output_types.get(ident)
    }

    /// Caches an input (object) type.
    pub fn cache_input_type(&mut self, ident: Identifier, typ: InputObjectTypeStrongRef) {
        self.cache.input_types.insert(ident, typ);
    }

    /// Caches an output (object) type.
    pub fn cache_output_type(&mut self, ident: Identifier, typ: ObjectTypeStrongRef) {
        self.cache.output_types.insert(ident, typ);
    }

    pub fn can_full_text_search(&self) -> bool {
        self.has_feature(&PreviewFeature::FullTextSearch)
            && (self.has_capability(ConnectorCapability::FullTextSearchWithoutIndex)
                || self.has_capability(ConnectorCapability::FullTextSearchWithIndex))
    }

    pub fn models(&self) -> Vec<ModelRef> {
        self.internal_data_model.models_cloned()
    }

    pub fn composite_types(&self) -> Vec<CompositeTypeRef> {
        self.internal_data_model.composite_types().to_owned()
    }
}

#[derive(Debug)]
struct TypeCache {
    input_types: TypeRefCache<InputObjectType>,
    output_types: TypeRefCache<ObjectType>,
}

impl TypeCache {
    pub fn new() -> Self {
        Self {
            input_types: TypeRefCache::new(),
            output_types: TypeRefCache::new(),
        }
    }

    /// Consumes the cache and collects all types to merge them into the vectors required to
    /// finalize the query schema building.
    /// Unwraps are safe because the cache is required to be the only strong Arc ref holder,
    /// which makes the Arc counter 1, all other refs contained in the schema are weak refs.
    pub fn collect_types(self) -> (Vec<InputObjectTypeStrongRef>, Vec<ObjectTypeStrongRef>) {
        let input_objects = self.input_types.into();
        let output_objects = self.output_types.into();

        (input_objects, output_objects)
    }
}

pub fn build(
    internal_data_model: InternalDataModelRef,
    enable_raw_queries: bool,
    capabilities: ConnectorCapabilities,
    preview_features: Vec<PreviewFeature>,
    referential_integrity: ReferentialIntegrity,
) -> QuerySchema {
    let mut ctx = BuilderContext::new(
        internal_data_model,
        enable_raw_queries,
        capabilities,
        preview_features.clone(),
    );

    output_types::objects::initialize_caches(&mut ctx);

    let (query_type, query_object_ref) = output_types::query_type::build(&mut ctx);
    let (mutation_type, mutation_object_ref) = output_types::mutation_type::build(&mut ctx);
    let (input_objects, mut output_objects) = ctx.cache.collect_types();

    // The mutation and query object types need to be part of the strong refs.
    output_objects.push(query_object_ref);
    output_objects.push(mutation_object_ref);

    let query_type = Arc::new(query_type);
    let mutation_type = Arc::new(mutation_type);

    QuerySchema::new(
        query_type,
        mutation_type,
        input_objects,
        output_objects,
        ctx.internal_data_model,
        ctx.capabilities.capabilities,
        preview_features,
        referential_integrity,
    )
}

type NestedInputsQueue = Vec<(Arc<InputObjectType>, RelationFieldRef)>;
