use super::*;
use crate::EnumType;
use constants::{filters, itx, json_null, load_strategy, ordering};
use psl::parser_database as db;
use query_structure::prelude::ParentContainer;

pub(crate) fn sort_order_enum() -> EnumType {
    let ident = Identifier::new_prisma(IdentifierType::SortOrder);

    EnumType::string(ident, vec![ordering::ASC.to_owned(), ordering::DESC.to_owned()])
}

pub(crate) fn nulls_order_enum() -> EnumType {
    EnumType::string(
        Identifier::new_prisma(ordering::NULLS_ORDER),
        vec![ordering::FIRST.to_owned(), ordering::LAST.to_owned()],
    )
}

pub(crate) fn map_schema_enum_type(ctx: &'_ QuerySchema, enum_id: db::EnumId) -> EnumType {
    let ident = Identifier::new_model(IdentifierType::Enum(ctx.internal_data_model.clone().zip(enum_id)));

    let schema_enum = ctx.internal_data_model.clone().zip(enum_id);
    EnumType::database(ident, schema_enum)
}

pub(crate) fn model_field_enum(model: &Model) -> EnumType {
    let ident = Identifier::new_prisma(IdentifierType::ScalarFieldEnum(model.clone()));

    let values = model
        .fields()
        .scalar()
        .map(|field| (field.name().to_owned(), field))
        .collect();

    EnumType::field_ref(ident, values)
}

pub(crate) fn json_null_filter_enum() -> EnumType {
    let ident = Identifier::new_prisma(json_null::FILTER_ENUM_NAME);

    EnumType::string(
        ident,
        vec![
            json_null::DB_NULL.to_owned(),
            json_null::JSON_NULL.to_owned(),
            json_null::ANY_NULL.to_owned(),
        ],
    )
}

pub(crate) fn json_null_input_enum(nullable: bool) -> EnumType {
    let ident = if nullable {
        Identifier::new_prisma(json_null::NULLABLE_INPUT_ENUM_NAME)
    } else {
        Identifier::new_prisma(json_null::INPUT_ENUM_NAME)
    };

    if nullable {
        EnumType::string(
            ident,
            vec![json_null::DB_NULL.to_owned(), json_null::JSON_NULL.to_owned()],
        )
    } else {
        EnumType::string(ident, vec![json_null::JSON_NULL.to_owned()])
    }
}

pub(crate) fn order_by_relevance_enum(container: ParentContainer, values: Vec<String>) -> EnumType {
    let ident = Identifier::new_prisma(IdentifierType::OrderByRelevanceFieldEnum(container));
    EnumType::string(ident, values)
}

pub(crate) fn query_mode_enum() -> EnumType {
    let ident = Identifier::new_prisma("QueryMode");
    EnumType::string(
        ident,
        vec![filters::DEFAULT.to_owned(), filters::INSENSITIVE.to_owned()],
    )
}

pub(crate) fn case_enum() -> EnumType {
    let ident = Identifier::new_prisma("Case");
    EnumType::string(
        ident,
        vec![filters::CASE_SENSITIVE.to_owned(), filters::CASE_INSENSITIVE.to_owned()],
    )
}

pub fn itx_isolation_levels(ctx: &'_ QuerySchema) -> Option<EnumType> {
    let ident = Identifier::new_prisma(IdentifierType::TransactionIsolationLevel);

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

    Some(EnumType::string(ident, values))
}

pub(crate) fn relation_load_strategy(ctx: &QuerySchema) -> Option<EnumType> {
    if !ctx.can_resolve_relation_with_joins() {
        return None;
    }

    let ident = Identifier::new_prisma(IdentifierType::RelationLoadStrategy);

    let values = if ctx.can_resolve_relation_with_joins() {
        vec![load_strategy::QUERY.to_owned(), load_strategy::JOIN.to_owned()]
    } else {
        vec![load_strategy::QUERY.to_owned()]
    };

    Some(EnumType::string(ident, values))
}
