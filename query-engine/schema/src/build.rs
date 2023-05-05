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

#[macro_use]
mod cache;
mod enum_types;
mod input_types;
mod mutations;
mod output_types;
mod utils;

pub use self::utils::{compound_id_field_name, compound_index_field_name};

use self::{enum_types::*, utils::*};
use crate::{db::QuerySchemaDatabase, *};
use cache::TypeRefCache;
use prisma_models::{ast, Field as ModelField, InternalDataModel, ModelRef, RelationFieldRef, TypeIdentifier};
use psl::{
    datamodel_connector::{Connector, ConnectorCapability},
    PreviewFeature, PreviewFeatures,
};

pub(crate) struct BuilderContext<'a> {
    internal_data_model: &'a InternalDataModel,
    enable_raw_queries: bool,
    db: QuerySchemaDatabase,
    input_types: TypeRefCache<InputObjectTypeId>,
    output_types: TypeRefCache<OutputObjectTypeId>,
    enum_types: TypeRefCache<EnumTypeId>,
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
        let input_types_estimate = models_count * 3;
        let output_types_estimate = models_count;
        let enum_types_estimate = 0; // not all connectors have enums
        let mut db = QuerySchemaDatabase::default();
        db.input_field_types
            .reserve(internal_data_model.schema.db.models_count() * 5);
        Self {
            internal_data_model,
            enable_raw_queries,
            input_types: TypeRefCache::with_capacity(input_types_estimate),
            output_types: TypeRefCache::with_capacity(output_types_estimate),
            enum_types: TypeRefCache::with_capacity(enum_types_estimate),
            db,
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
    fn get_input_type(&mut self, ident: &Identifier) -> Option<InputObjectTypeId> {
        self.input_types.get(ident)
    }

    /// Get an output (object) type.
    pub(crate) fn get_output_type(&mut self, ident: &Identifier) -> Option<OutputObjectTypeId> {
        self.output_types.get(ident)
    }

    /// Get an enum type.
    pub(crate) fn get_enum_type(&mut self, ident: &Identifier) -> Option<EnumTypeId> {
        self.enum_types.get(ident)
    }

    /// Caches an input (object) type.
    pub(crate) fn cache_input_type(&mut self, ident: Identifier, typ: InputObjectType) -> InputObjectTypeId {
        let id = self.db.push_input_object_type(typ);
        self.input_types.insert(ident, id);
        id
    }

    /// Caches an output (object) type.
    pub fn cache_output_type(&mut self, ident: Identifier, typ: ObjectType) -> OutputObjectTypeId {
        let id = self.db.push_output_object_type(typ);
        self.output_types.insert(ident, id);
        id
    }

    /// Caches an enum type.
    pub fn cache_enum_type(&mut self, ident: Identifier, e: EnumType) -> EnumTypeId {
        let id = self.db.push_enum_type(e);
        self.enum_types.insert(ident, id);
        id
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

pub fn build(schema: Arc<psl::ValidatedSchema>, enable_raw_queries: bool) -> QuerySchema {
    let _span = tracing::info_span!("prisma:engine:schema").entered();
    let preview_features = schema.configuration.preview_features();
    build_with_features(schema, preview_features, enable_raw_queries)
}

pub fn build_with_features(
    schema: Arc<psl::ValidatedSchema>,
    preview_features: PreviewFeatures,
    enable_raw_queries: bool,
) -> QuerySchema {
    let internal_data_model = prisma_models::convert(schema);
    let mut ctx = BuilderContext::new(&internal_data_model, enable_raw_queries, preview_features);

    output_types::objects::initialize_caches(&mut ctx);

    let query_type = output_types::query_type::build(&mut ctx);
    let mutation_type = output_types::mutation_type::build(&mut ctx);

    // Add iTX isolation levels to the schema.
    enum_types::itx_isolation_levels(&mut ctx);

    let capabilities = ctx.connector.capabilities().to_owned();

    QuerySchema::new(query_type, mutation_type, ctx.db, internal_data_model, capabilities)
}

type NestedInputsQueue = Vec<(InputObjectTypeId, RelationFieldRef)>;
