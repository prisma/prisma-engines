use super::*;
use crate::constants::{filters, itx, json_null, ordering};
use schema::EnumType;

pub(crate) fn sort_order_enum(ctx: &mut BuilderContext) -> EnumTypeWeakRef {
    let ident = Identifier::new(ordering::SORT_ORDER, PRISMA_NAMESPACE);
    return_cached_enum!(ctx, &ident);

    let typ = Arc::new(EnumType::string(
        ident.clone(),
        vec![ordering::ASC.to_owned(), ordering::DESC.to_owned()],
    ));

    ctx.cache_enum_type(ident, typ.clone());
    Arc::downgrade(&typ)
}

pub(crate) fn nulls_order_enum(ctx: &mut BuilderContext) -> EnumTypeWeakRef {
    let ident = Identifier::new(ordering::NULLS_ORDER, PRISMA_NAMESPACE);
    return_cached_enum!(ctx, &ident);

    let typ = Arc::new(EnumType::string(
        ident.clone(),
        vec![ordering::FIRST.to_owned(), ordering::LAST.to_owned()],
    ));

    ctx.cache_enum_type(ident, typ.clone());
    Arc::downgrade(&typ)
}

pub(crate) fn map_schema_enum_type(ctx: &mut BuilderContext, enum_name: &str) -> EnumTypeWeakRef {
    let ident = Identifier::new(enum_name, MODEL_NAMESPACE);
    return_cached_enum!(ctx, &ident);

    let schema_enum = ctx
        .internal_data_model
        .find_enum(enum_name)
        .expect("Enum references must always be valid.");

    let typ = Arc::new(EnumType::database(ident.clone(), schema_enum));

    ctx.cache_enum_type(ident, typ.clone());
    Arc::downgrade(&typ)
}

pub(crate) fn model_field_enum(ctx: &mut BuilderContext, model: &ModelRef) -> EnumTypeWeakRef {
    let name = format!("{}ScalarFieldEnum", capitalize(&model.name));
    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_enum!(ctx, &ident);

    let values = model
        .fields()
        .scalar()
        .into_iter()
        .map(|field| (field.name.clone(), field))
        .collect();

    let typ = Arc::new(EnumType::field_ref(ident.clone(), values));

    ctx.cache_enum_type(ident, typ.clone());
    Arc::downgrade(&typ)
}

pub(crate) fn json_null_filter_enum(ctx: &mut BuilderContext) -> EnumTypeWeakRef {
    let ident = Identifier::new(json_null::FILTER_ENUM_NAME, PRISMA_NAMESPACE);
    return_cached_enum!(ctx, &ident);

    let typ = Arc::new(EnumType::string(
        ident.clone(),
        vec![
            json_null::DB_NULL.to_owned(),
            json_null::JSON_NULL.to_owned(),
            json_null::ANY_NULL.to_owned(),
        ],
    ));

    ctx.cache_enum_type(ident, typ.clone());
    Arc::downgrade(&typ)
}

pub(crate) fn json_null_input_enum(ctx: &mut BuilderContext, nullable: bool) -> EnumTypeWeakRef {
    let ident = if nullable {
        Identifier::new(json_null::NULLABLE_INPUT_ENUM_NAME, PRISMA_NAMESPACE)
    } else {
        Identifier::new(json_null::INPUT_ENUM_NAME, PRISMA_NAMESPACE)
    };

    return_cached_enum!(ctx, &ident);

    let typ = if nullable {
        Arc::new(EnumType::string(
            ident.clone(),
            vec![json_null::DB_NULL.to_owned(), json_null::JSON_NULL.to_owned()],
        ))
    } else {
        Arc::new(EnumType::string(ident.clone(), vec![json_null::JSON_NULL.to_owned()]))
    };

    ctx.cache_enum_type(ident, typ.clone());
    Arc::downgrade(&typ)
}

pub(crate) fn order_by_relevance_enum(
    ctx: &mut BuilderContext,
    container: &str,
    values: Vec<String>,
) -> EnumTypeWeakRef {
    let name = format!("{}OrderByRelevanceFieldEnum", container);
    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_enum!(ctx, &ident);

    let typ = Arc::new(EnumType::string(ident.clone(), values));

    ctx.cache_enum_type(ident, typ.clone());
    Arc::downgrade(&typ)
}

pub(crate) fn query_mode_enum(ctx: &mut BuilderContext) -> EnumTypeWeakRef {
    let ident = Identifier::new("QueryMode", PRISMA_NAMESPACE);
    return_cached_enum!(ctx, &ident);

    let typ = Arc::new(EnumType::string(
        ident.clone(),
        vec![filters::DEFAULT.to_owned(), filters::INSENSITIVE.to_owned()],
    ));

    ctx.cache_enum_type(ident, typ.clone());
    Arc::downgrade(&typ)
}

pub(crate) fn itx_isolation_levels(ctx: &mut BuilderContext) -> Option<EnumTypeWeakRef> {
    let ident = Identifier::new("TransactionIsolationLevel", PRISMA_NAMESPACE);
    if let e @ Some(_) = ctx.get_enum_type(&ident) {
        return e;
    }

    let mut values = vec![];

    if ctx.has_capability(ConnectorCapability::SupportsTxIsolationReadUncommitted) {
        values.push(itx::READ_UNCOMMITTED.to_owned());
    }

    if ctx.has_capability(ConnectorCapability::SupportsTxIsolationReadCommitted) {
        values.push(itx::READ_COMMITTED.to_owned());
    }

    if ctx.has_capability(ConnectorCapability::SupportsTxIsolationRepeatableRead) {
        values.push(itx::REPEATABLE_READ.to_owned());
    }

    if ctx.has_capability(ConnectorCapability::SupportsTxIsolationSerializable) {
        values.push(itx::SERIALIZABLE.to_owned());
    }

    if ctx.has_capability(ConnectorCapability::SupportsTxIsolationSnapshot) {
        values.push(itx::SNAPSHOT.to_owned());
    }

    if values.is_empty() {
        return None;
    }

    let typ = Arc::new(EnumType::string(ident.clone(), values));
    ctx.cache_enum_type(ident, typ.clone());

    Some(Arc::downgrade(&typ))
}
