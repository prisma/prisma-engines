use diagnostics::DatamodelError;

use crate::{Datasource, validate::validation_pipeline::context::Context};

pub(super) fn schemas_property_with_no_connector_support(datasource: &Datasource, ctx: &mut Context<'_>) {
    if ctx.has_capability(crate::datamodel_connector::ConnectorCapability::MultiSchema) {
        return;
    }

    if let Some(span) = datasource.schemas_span {
        ctx.push_error(DatamodelError::new_static(
            "The `schemas` property is not supported on the current connector.",
            span,
        ))
    }
}
