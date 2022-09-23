use crate::{ast, validate::validation_pipeline::context::Context};

pub(super) fn validate_db_name(
    model_name: &str,
    attribute: &ast::Attribute,
    db_name: Option<&str>,
    ctx: &mut Context<'_>,
    // How many @ in the error message?
    double_at: bool,
) {
    if let Some(err) = crate::datamodel_connector::constraint_names::ConstraintNames::is_db_name_too_long(
        attribute.span,
        model_name,
        db_name,
        &attribute.name.name,
        ctx.connector,
        double_at,
    ) {
        ctx.push_error(err);
    }
}
