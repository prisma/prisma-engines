use crate::transform::ast_to_dml::db::walkers::CompositeTypeWalker;
use datamodel_connector::Connector;
use diagnostics::{DatamodelError, Diagnostics};

pub(crate) fn composite_types_support(
    composite_type: CompositeTypeWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    if connector.supports_composite_types() {
        return;
    }

    diagnostics.push_error(DatamodelError::new_validation_error(
        format!("Composite types are not supported on {}.", connector.name()),
        composite_type.ast_composite_type().span,
    ));
}
