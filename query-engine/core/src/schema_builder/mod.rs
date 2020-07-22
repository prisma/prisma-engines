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

mod arguments;
mod filter_arguments;
mod mutation_type;
mod query_type;
mod utils;

use std::collections::HashMap;

use crate::schema::*;
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
            cache: TypeCache::default(),
        }
    }

    // Just here for convenience, will be removed soon.
    pub fn pluralize_internal(&self, legacy: String, modern: String) -> String {
        match self.mode {
            BuildMode::Legacy => legacy,
            BuildMode::Modern => modern,
        }
    }
}

#[derive(Default)]
struct TypeCache {
    input_types: HashMap<String, InputObjectTypeStrongRef>,
    output_types: HashMap<String, ObjectTypeStrongRef>,
}

impl TypeCache {
    /// Consumes the cache and collects all types to merge them into the vectors required to
    /// finalize the query schema building.
    /// Unwraps are safe because the cache is required to be the only strong Arc ref holder,
    /// which makes the Arc counter 1, all other refs contained in the schema are weak refs.
    pub fn collect_types(self) -> (Vec<InputObjectTypeStrongRef>, Vec<ObjectTypeStrongRef>) {
        let input_objects = self.input_types.into_iter().map(|(_, v)| v).collect();
        let output_objects = self.output_types.into_iter().map(|(_, v)| v).collect();

        (input_objects, output_objects)
    }
}

pub fn build(internal_data_model: InternalDataModelRef, mode: BuildMode, enable_raw_queries: bool) -> QuerySchema {
    let mut ctx = BuilderContext::new(mode, internal_data_model, enable_raw_queries);
    let (query_type, query_object_ref) = query_type::build(&mut ctx);
    let (mutation_type, mutation_object_ref) = mutation_type::build(&mut ctx);
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
