use super::*;
use constants::{filters, itx, json_null, ordering};
use prisma_models::prelude::ParentContainer;
use schema::{EnumType, EnumTypeId};

pub(crate) fn sort_order_enum(ctx: &mut BuilderContext<'_>) -> EnumTypeId {
    let ident = Identifier::new_prisma(ordering::SORT_ORDER);
    return_cached_enum!(ctx, &ident);

    let typ = EnumType::string(ident.clone(), vec![ordering::ASC.to_owned(), ordering::DESC.to_owned()]);

    ctx.cache_enum_type(ident, typ)
}

pub(crate) fn nulls_order_enum(ctx: &mut BuilderContext<'_>) -> EnumTypeId {
    let ident = Identifier::new_prisma(ordering::NULLS_ORDER);
    return_cached_enum!(ctx, &ident);

    let typ = EnumType::string(
        ident.clone(),
        vec![ordering::FIRST.to_owned(), ordering::LAST.to_owned()],
    );

    ctx.cache_enum_type(ident, typ)
}

pub(crate) fn map_schema_enum_type(ctx: &mut BuilderContext<'_>, enum_id: ast::EnumId) -> EnumTypeId {
    let ident = Identifier::new_model(IdentifierType::Enum(ctx.internal_data_model.clone().zip(enum_id)));
    return_cached_enum!(ctx, &ident);

    let schema_enum = ctx.internal_data_model.clone().zip(enum_id);
    let typ = EnumType::database(ident.clone(), schema_enum);
    ctx.cache_enum_type(ident, typ)
}

pub(crate) fn model_field_enum(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> EnumTypeId {
    let ident = Identifier::new_prisma(IdentifierType::ScalarFieldEnum(model.clone()));
    return_cached_enum!(ctx, &ident);

    let values = model
        .fields()
        .scalar()
        .into_iter()
        .map(|field| (field.name().to_owned(), field))
        .collect();

    let typ = EnumType::field_ref(ident.clone(), values);
    ctx.cache_enum_type(ident, typ)
}

pub(crate) fn json_null_filter_enum(ctx: &mut BuilderContext<'_>) -> EnumTypeId {
    let ident = Identifier::new_prisma(json_null::FILTER_ENUM_NAME);
    return_cached_enum!(ctx, &ident);

    let typ = EnumType::string(
        ident.clone(),
        vec![
            json_null::DB_NULL.to_owned(),
            json_null::JSON_NULL.to_owned(),
            json_null::ANY_NULL.to_owned(),
        ],
    );

    ctx.cache_enum_type(ident, typ)
}

pub(crate) fn json_null_input_enum(ctx: &mut BuilderContext<'_>, nullable: bool) -> EnumTypeId {
    let ident = if nullable {
        Identifier::new_prisma(json_null::NULLABLE_INPUT_ENUM_NAME)
    } else {
        Identifier::new_prisma(json_null::INPUT_ENUM_NAME)
    };

    return_cached_enum!(ctx, &ident);

    let typ = if nullable {
        EnumType::string(
            ident.clone(),
            vec![json_null::DB_NULL.to_owned(), json_null::JSON_NULL.to_owned()],
        )
    } else {
        EnumType::string(ident.clone(), vec![json_null::JSON_NULL.to_owned()])
    };

    ctx.cache_enum_type(ident, typ)
}

pub(crate) fn order_by_relevance_enum(
    ctx: &mut BuilderContext<'_>,
    container: &ParentContainer,
    values: Vec<String>,
) -> EnumTypeId {
    let ident = Identifier::new_prisma(IdentifierType::OrderByRelevanceFieldEnum(container.clone()));
    return_cached_enum!(ctx, &ident);

    let typ = EnumType::string(ident.clone(), values);

    ctx.cache_enum_type(ident, typ)
}

pub(crate) fn query_mode_enum(ctx: &mut BuilderContext<'_>) -> EnumTypeId {
    let ident = Identifier::new_prisma("QueryMode");
    return_cached_enum!(ctx, &ident);

    let typ = EnumType::string(
        ident.clone(),
        vec![filters::DEFAULT.to_owned(), filters::INSENSITIVE.to_owned()],
    );

    ctx.cache_enum_type(ident, typ)
}

pub(crate) fn itx_isolation_levels(ctx: &mut BuilderContext<'_>) -> Option<EnumTypeId> {
    let ident = Identifier::new_prisma(IdentifierType::TransactionIsolationLevel);
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

    let typ = EnumType::string(ident.clone(), values);
    Some(ctx.cache_enum_type(ident, typ))
}
