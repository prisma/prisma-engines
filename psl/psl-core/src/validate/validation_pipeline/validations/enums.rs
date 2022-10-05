use crate::{
    datamodel_connector::{Connector, ConnectorCapability},
    diagnostics::DatamodelError,
    parser_database::{self, walkers::EnumWalker},
    validate::validation_pipeline::context::Context,
};

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

    if !ctx
        .db
        .schema_flags()
        .contains(parser_database::SchemaFlags::UsesSchemaAttribute)
    {
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

pub(crate) fn connector_supports_enums(r#enum: EnumWalker<'_>, connector: &dyn Connector, ctx: &mut Context<'_>) {
    if connector.supports_enums() {
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
