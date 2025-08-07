use diagnostics::DatamodelError;
use parser_database::{
    ast::WithSpan,
    walkers::{IndexWalker, ModelWalker, PrimaryKeyWalker, ScalarFieldAttributeWalker},
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

pub(super) fn index(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if !index.is_unique() {
        ctx.push_error(DatamodelError::new_validation_error(
            "Views cannot have indexes.",
            index.ast_attribute().span,
        ));
    }

    if index.mapped_name().is_some() {
        ctx.push_error(DatamodelError::new_validation_error(
            "@@unique annotations on views are not backed by unique indexes in the database and cannot specify a mapped database name.",
            index.ast_attribute().span,
        ));
    }

    if index.clustered().is_some() {
        ctx.push_error(DatamodelError::new_validation_error(
            "@@unique annotations on views are not backed by unique indexes in the database and cannot be clustered.",
            index.ast_attribute().span,
        ));
    }
}

pub(super) fn index_field_attribute(
    index: IndexWalker<'_>,
    attr: ScalarFieldAttributeWalker<'_>,
    ctx: &mut Context<'_>,
) {
    if attr.length().is_some() || attr.operator_class().is_some() || attr.sort_order().is_some() {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            "Scalar fields in @@unique attributes in views cannot have arguments.",
            index.attribute_name(),
            index.ast_attribute().span,
        ));
    }
}

pub(super) fn connector_specific(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    ctx.connector.validate_view(model, ctx.diagnostics)
}
