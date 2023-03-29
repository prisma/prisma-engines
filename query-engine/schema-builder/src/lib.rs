#![deny(rust_2018_idioms, unsafe_code)]

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

pub mod constants;

#[macro_use]
mod cache;
mod enum_types;
mod input_types;
mod mutations;
mod output_types;
mod utils;

pub use self::utils::{compound_id_field_name, compound_index_field_name};

use cache::TypeRefCache;
use prisma_models::{ast, Field as ModelField, InternalDataModel, ModelRef, RelationFieldRef, TypeIdentifier};
use psl::{
    datamodel_connector::{Connector, ConnectorCapability},
    PreviewFeature, PreviewFeatures,
};
use schema::*;
use std::sync::Arc;
use utils::*;

pub(crate) struct BuilderContext<'a> {
    pub(crate) input_field_types: Vec<InputType>,
    internal_data_model: &'a InternalDataModel,
    enable_raw_queries: bool,
    input_types: TypeRefCache<InputObjectType>,
    output_types: TypeRefCache<ObjectType>,
    enum_types: TypeRefCache<EnumType>,
    connector: &'static dyn Connector,
    preview_features: PreviewFeatures,
    nested_create_inputs_queue: NestedInputsQueue,
    nested_update_inputs_queue: NestedInputsQueue,
}

impl<'a> BuilderContext<'a> {
    fn new(
        internal_data_model: &'a InternalDataModel,
        enable_raw_queries: bool,
        preview_features: PreviewFeatures,
    ) -> Self {
        let connector = internal_data_model.schema.connector;
        let models_count = internal_data_model.schema.db.models_count();
        Self {
            input_field_types: Vec::with_capacity(internal_data_model.schema.db.models_count() * 5),
            internal_data_model,
            enable_raw_queries,
            input_types: TypeRefCache::with_capacity(models_count * 3),
            output_types: TypeRefCache::with_capacity(models_count),
            enum_types: TypeRefCache::with_capacity(0),
            connector,
            preview_features,
            nested_create_inputs_queue: Vec::new(),
            nested_update_inputs_queue: Vec::new(),
        }
    }

    fn has_feature(&self, feature: PreviewFeature) -> bool {
        self.preview_features.contains(feature)
    }

    fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.connector.has_capability(capability)
    }

    /// Get an input (object) type.
    fn get_input_type(&mut self, ident: &Identifier) -> Option<InputObjectTypeWeakRef> {
        self.input_types.get(ident)
    }

    /// Get an output (object) type.
    pub(crate) fn get_output_type(&mut self, ident: &Identifier) -> Option<ObjectTypeWeakRef> {
        self.output_types.get(ident)
    }

    /// Get an enum type.
    pub(crate) fn get_enum_type(&mut self, ident: &Identifier) -> Option<EnumTypeWeakRef> {
        self.enum_types.get(ident)
    }

    /// Caches an input (object) type.
    pub(crate) fn cache_input_type(&mut self, ident: Identifier, typ: InputObjectTypeStrongRef) {
        self.input_types.insert(ident, typ);
    }

    /// Caches an output (object) type.
    pub fn cache_output_type(&mut self, ident: Identifier, typ: ObjectTypeStrongRef) {
        self.output_types.insert(ident, typ);
    }

    /// Caches an enum type.
    pub fn cache_enum_type(&mut self, ident: Identifier, e: EnumTypeRef) {
        self.enum_types.insert(ident, e);
    }

    pub fn can_full_text_search(&self) -> bool {
        self.has_feature(PreviewFeature::FullTextSearch)
            && (self.has_capability(ConnectorCapability::FullTextSearchWithoutIndex)
                || self.has_capability(ConnectorCapability::FullTextSearchWithIndex))
    }

    pub fn supports_any(&self, capabilities: &[ConnectorCapability]) -> bool {
        capabilities.iter().any(|c| self.connector.has_capability(*c))
    }
}

pub fn build(internal_data_model: InternalDataModel, enable_raw_queries: bool) -> QuerySchema {
    let preview_features = internal_data_model.schema.configuration.preview_features();
    build_with_features(internal_data_model, preview_features, enable_raw_queries)
}

pub fn build_with_features(
    internal_data_model: InternalDataModel,
    preview_features: PreviewFeatures,
    enable_raw_queries: bool,
) -> QuerySchema {
    let mut ctx = BuilderContext::new(&internal_data_model, enable_raw_queries, preview_features);

    output_types::objects::initialize_caches(&mut ctx);

    let (query_type, query_object_ref) = output_types::query_type::build(&mut ctx);
    let (mutation_type, mutation_object_ref) = output_types::mutation_type::build(&mut ctx);

    // Add iTX isolation levels to the schema.
    enum_types::itx_isolation_levels(&mut ctx);

    let input_objects = ctx.input_types.into();
    let mut output_objects: Vec<_> = ctx.output_types.into();
    let enum_types = ctx.enum_types.into();

    // The mutation and query object types need to be part of the strong refs.
    output_objects.push(query_object_ref);
    output_objects.push(mutation_object_ref);

    let query_type = Arc::new(query_type);
    let mutation_type = Arc::new(mutation_type);
    let capabilities = ctx.connector.capabilities().to_owned();
    let input_field_types = ctx.input_field_types;

    QuerySchema::new(
        query_type,
        mutation_type,
        input_field_types,
        input_objects,
        output_objects,
        enum_types,
        internal_data_model,
        capabilities,
    )
}

type NestedInputsQueue = Vec<(Arc<InputObjectType>, RelationFieldRef)>;
