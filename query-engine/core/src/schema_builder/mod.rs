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
mod arguments;
mod input_types;
mod output_types;
mod utils;

use crate::schema::*;
use cache::TypeRefCache;
use prisma_models::{Field as ModelField, Index, InternalDataModelRef, ModelRef, TypeIdentifier};
use std::sync::Arc;

pub use utils::*;

// [DTODO] Remove
/// Build mode for schema generation.
#[derive(Debug, Copy, Clone)]
pub enum BuildMode {
    /// Prisma 1 compatible schema generation.
    /// This will still generate only a subset of the legacy schema.
    Legacy,

    /// Prisma 2 schema. Uses different inflection strategy
    Modern,
}

pub(crate) struct BuilderContext {
    mode: BuildMode,
    internal_data_model: InternalDataModelRef,
    enable_raw_queries: bool,
    cache: TypeCache,
}

impl BuilderContext {
    pub fn new(mode: BuildMode, internal_data_model: InternalDataModelRef, enable_raw_queries: bool) -> Self {
        Self {
            mode,
            internal_data_model,
            enable_raw_queries,
            cache: TypeCache::new(),
        }
    }

    // Just here for convenience, will be removed soon.
    pub fn pluralize_internal(&self, legacy: String, modern: String) -> String {
        match self.mode {
            BuildMode::Legacy => legacy,
            BuildMode::Modern => modern,
        }
    }

    /// Get an input (object) type.
    pub fn get_input_type(&mut self, name: &str) -> Option<InputObjectTypeWeakRef> {
        self.cache.input_types.get(name)
    }

    /// Get an output (object) type.
    pub fn get_output_type(&mut self, name: &str) -> Option<ObjectTypeWeakRef> {
        self.cache.output_types.get(name)
    }

    /// Caches an input (object) type.
    pub fn cache_input_type(&mut self, name: String, typ: InputObjectTypeStrongRef) {
        self.cache.input_types.insert(name, typ);
    }

    /// Caches an output (object) type.
    pub fn cache_output_type(&mut self, name: String, typ: ObjectTypeStrongRef) {
        self.cache.output_types.insert(name, typ);
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

pub fn build(internal_data_model: InternalDataModelRef, mode: BuildMode, enable_raw_queries: bool) -> QuerySchema {
    let mut ctx = BuilderContext::new(mode, internal_data_model, enable_raw_queries);
    output_types::output_objects::initialize_model_object_type_cache(&mut ctx);

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
    )
}
