pub mod composite;
pub mod model;

use super::*;
use crate::constants::output_fields::*;

/// Initializes output object type caches on the context.
/// This is a critical first step to ensure that all model and composite output
/// object types are present and that subsequent schema computation has a base to rely on.
/// Called only once at the very beginning of schema building.
pub(crate) fn initialize_caches(ctx: &mut BuilderContext) {
    model::initialize_cache(ctx);
    composite::initialize_cache(ctx);

    model::initialize_fields(ctx);
    composite::initialize_fields(ctx);
}

pub(crate) fn affected_records_object_type(ctx: &mut BuilderContext) -> ObjectTypeWeakRef {
    let ident = Identifier::new("AffectedRowsOutput".to_owned(), PRISMA_NAMESPACE);
    return_cached_output!(ctx, &ident);

    let object_type = Arc::new(object_type(
        ident.clone(),
        vec![field(AFFECTED_COUNT, vec![], OutputType::int(), None)],
        None,
    ));

    ctx.cache_output_type(ident, object_type.clone());
    Arc::downgrade(&object_type)
}
