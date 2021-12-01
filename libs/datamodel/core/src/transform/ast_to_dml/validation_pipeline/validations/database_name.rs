use crate::{ast, common::constraint_names::ConstraintNames, Diagnostics};
use datamodel_connector::Connector;

pub(super) fn validate_db_name(
    model_name: &str,
    attribute: &ast::Attribute,
    db_name: Option<&str>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
    // How many @ in the error message?
    double_at: bool,
) {
    if let Some(err) = ConstraintNames::is_db_name_too_long(
        attribute.span,
        model_name,
        db_name,
        &attribute.name.name,
        connector,
        double_at,
    ) {
        diagnostics.push_error(err);
    }
}
