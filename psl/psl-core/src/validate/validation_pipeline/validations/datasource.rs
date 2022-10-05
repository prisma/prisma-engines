use diagnostics::DatamodelError;

use crate::{validate::validation_pipeline::context::Context, Datasource};

pub(super) fn schemas_property_without_preview_feature(datasource: &Datasource, ctx: &mut Context<'_>) {
    if ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if let Some(span) = datasource.schemas_span {
        ctx.push_error(DatamodelError::new_static(
            "The `schemas` property is only availably with the `multiSchema` preview feature.",
            span,
        ))
    }
}

pub(super) fn schemas_property_with_no_connector_support(datasource: &Datasource, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if ctx
        .connector
        .has_capability(crate::datamodel_connector::ConnectorCapability::MultiSchema)
    {
        return;
    }

    if let Some(span) = datasource.schemas_span {
        ctx.push_error(DatamodelError::new_static(
            "The `schemas` property is not supported on the current connector.",
            span,
        ))
    }
}

pub(super) fn extensions_property_without_preview_feature(datasource: &Datasource, ctx: &mut Context<'_>) {
    if ctx.preview_features.contains(crate::PreviewFeature::PostgresExtensions) {
        return;
    }

    if let Some(span) = datasource.extensions_span {
        ctx.push_error(DatamodelError::new_static(
            "The `extensions` property is only available with the `postgresExtensions` preview feature.",
            span,
        ));
    }
}

pub(crate) fn extensions_property_with_no_connector_support(ds: &Datasource, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(crate::PreviewFeature::PostgresExtensions) {
        return;
    }

    if ctx.connector.is_provider("postgresql") {
        return;
    }

    if let Some(span) = ds.extensions_span {
        ctx.push_error(DatamodelError::new_static(
            "The `extensions` property is only available with the `postgresql` connector.",
            span,
        ));
    }
}
