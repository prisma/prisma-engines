use crate::{ast, coerce, context::*, types::*, StringId};

pub(super) fn model(model_attributes: &mut ModelAttributes, ctx: &mut Context<'_>) {
    model_attributes.schema = visit_schema_attribute(ctx);
}

pub(super) fn r#enum(enum_attributes: &mut EnumAttributes, ctx: &mut Context<'_>) {
    enum_attributes.schema = visit_schema_attribute(ctx);
}

fn visit_schema_attribute(ctx: &mut Context<'_>) -> Option<(StringId, ast::Span)> {
    let arg = match ctx.visit_default_arg("map") {
        Ok(arg) => arg,
        Err(err) => {
            ctx.push_error(err);
            return None;
        }
    };
    let name = coerce::string(arg, ctx.diagnostics)?;
    Some((ctx.interner.intern(name), arg.span()))
}
