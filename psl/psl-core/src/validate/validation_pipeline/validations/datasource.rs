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
