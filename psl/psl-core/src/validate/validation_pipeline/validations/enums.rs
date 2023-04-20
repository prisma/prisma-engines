use crate::{
    datamodel_connector::ConnectorCapability,
    diagnostics::DatamodelError,
    parser_database::{ast::WithSpan, walkers::EnumWalker},
    validate::validation_pipeline::context::Context,
};
use std::collections::HashSet;

pub(super) fn database_name_clashes(ctx: &mut Context<'_>) {
    let mut database_names: HashSet<(Option<&str>, &str)> = HashSet::with_capacity(ctx.db.enums_count());

    for enm in ctx.db.walk_enums() {
        if !database_names.insert((enm.schema().map(|(n, _)| n), enm.database_name())) {
            ctx.push_error(DatamodelError::new_duplicate_enum_database_name_error(
                enm.ast_enum().span(),
            ));
        }
    }
}

pub(super) fn schema_is_defined_in_the_datasource(r#enum: EnumWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if !ctx.connector.has_capability(ConnectorCapability::MultiSchema) {
        return;
    }

    let datasource = match ctx.datasource {
        Some(ds) => ds,
        None => return,
    };

    let (schema_name, span) = match r#enum.schema() {
        Some(tuple) => tuple,
        None => return,
    };

    if datasource.has_schema(schema_name) {
        return;
    }

    ctx.push_error(DatamodelError::new_static(
        "This schema is not defined in the datasource. Read more on `@@schema` at https://pris.ly/d/multi-schema",
        span,
    ))
}

pub(super) fn schema_attribute_supported_in_connector(r#enum: EnumWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if ctx.connector.has_capability(ConnectorCapability::MultiSchema) {
        return;
    }

    let (_, span) = match r#enum.schema() {
        Some(tuple) => tuple,
        None => return,
    };

    ctx.push_error(DatamodelError::new_static(
        "@@schema is not supported on the current datasource provider",
        span,
    ));
}

pub(super) fn schema_attribute_missing(r#enum: EnumWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if !ctx.connector.has_capability(ConnectorCapability::MultiSchema) {
        return;
    }

    let datasource = match ctx.datasource {
        Some(datasource) => datasource,
        None => return,
    };

    if datasource.schemas_span.is_none() {
        return;
    }

    if ctx.connector.is_provider("mysql") {
        return;
    }

    if r#enum.schema().is_some() {
        return;
    }

    ctx.push_error(DatamodelError::new_static(
        "This enum is missing an `@@schema` attribute.",
        r#enum.ast_enum().span,
    ))
}

pub(super) fn multischema_feature_flag_needed(r#enum: EnumWalker<'_>, ctx: &mut Context<'_>) {
    if ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if let Some((_, span)) = r#enum.schema() {
        ctx.push_error(DatamodelError::new_static(
            "@@schema is only available with the `multiSchema` preview feature.",
            span,
        ));
    }
}

pub(crate) fn connector_supports_enums(r#enum: EnumWalker<'_>, ctx: &mut Context<'_>) {
    if ctx.connector.supports_enums() {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        &format!(
            "You defined the enum `{}`. But the current connector does not support enums.",
            r#enum.name()
        ),
        r#enum.ast_enum().span,
    ));
}
