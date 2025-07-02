use diagnostics::DatamodelError;

use crate::{validate::validation_pipeline::context::Context, Datasource};

pub(super) fn namespaces_property_without_preview_feature(datasource: &Datasource, ctx: &mut Context<'_>) {
    if ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if let Some(span) = datasource.namespaces_span {
        ctx.push_error(DatamodelError::new_static(
            "The `namespaces` property is only available with the `multiSchema` preview feature.",
            span,
        ))
    }
}

pub(super) fn namespaces_property_with_no_connector_support(datasource: &Datasource, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if ctx.has_capability(crate::datamodel_connector::ConnectorCapability::MultiNamespace) {
        return;
    }

    if let Some(span) = datasource.namespaces_span {
        ctx.push_error(DatamodelError::new_static(
            "The `namespaces` property is not supported on the current connector.",
            span,
        ))
    }
}
