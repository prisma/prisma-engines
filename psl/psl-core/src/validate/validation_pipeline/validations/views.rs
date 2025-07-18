use diagnostics::DatamodelError;
use parser_database::{
    ast::WithSpan,
    walkers::{IndexWalker, ModelWalker, PrimaryKeyWalker},
};

use crate::validate::validation_pipeline::context::Context;

pub(crate) fn view_definition_without_preview_flag(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    if ctx.preview_features.contains(crate::PreviewFeature::Views) {
        return;
    }

    if !model.ast_model().is_view() {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        "View definitions are only available with the `views` preview feature.",
        model.ast_model().span(),
    ));
}

pub(super) fn primary_key(model: PrimaryKeyWalker<'_>, ctx: &mut Context<'_>) {
    ctx.push_error(DatamodelError::new_validation_error(
        "Views cannot have primary keys.",
        model.ast_attribute().span,
    ));
}

pub(super) fn index(model: IndexWalker<'_>, ctx: &mut Context<'_>) {
    ctx.push_error(DatamodelError::new_validation_error(
        "Views cannot have indexes.",
        model.ast_attribute().span,
    ));
}

pub(super) fn connector_specific(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    ctx.connector.validate_view(model, ctx.diagnostics)
}
