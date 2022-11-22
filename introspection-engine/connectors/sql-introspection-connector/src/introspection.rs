pub(crate) mod inline_relations;

mod configuration;
mod enums;
mod indexes;
mod m2m_relations;
mod models;
mod postgres;
mod prisma_relation_mode;
mod relation_names;

use crate::{calculate_datamodel::CalculateDatamodelContext as Context, SqlError};

pub(crate) fn introspect(ctx: &mut Context) -> Result<(String, bool), SqlError> {
    enums::render(ctx);
    models::render(ctx);

    let relation_names = relation_names::introspect_relation_names(ctx);

    if ctx.foreign_keys_enabled() {
        inline_relations::render(&relation_names, ctx);
    } else {
        prisma_relation_mode::render(ctx);
    }

    m2m_relations::render(&relation_names, ctx);

    let rendered = if ctx.render_config {
        format!(
            "{}\n{}",
            configuration::render(ctx.config, ctx.schema),
            ctx.rendered_schema
        )
    } else {
        ctx.rendered_schema.to_string()
    };

    ctx.finalize_warnings();

    Ok((psl::reformat(&rendered, 2).unwrap(), ctx.rendered_schema.is_empty()))
}
