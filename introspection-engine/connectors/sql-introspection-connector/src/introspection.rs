pub(crate) mod inline_relations;

mod configuration;
mod enums;
mod indexes;
mod m2m_relations;
mod models;
mod postgres;
mod prisma_relation_mode;
mod relation_names;
mod scalar_field;

use crate::calculate_datamodel::{InputContext, OutputContext};
pub(crate) use crate::SqlError;

pub(crate) fn introspect<'a>(
    input: InputContext<'a>,
    output: &mut OutputContext<'a>,
) -> Result<(String, bool), SqlError> {
    enums::render(input, output);
    models::render(input, output);

    if input.foreign_keys_enabled() {
        let relation_names = relation_names::introspect(input);

        inline_relations::render(&relation_names, input, output);
        m2m_relations::render(&relation_names, input, output);
    } else {
        prisma_relation_mode::render(input, output);
    }

    let rendered = if input.render_config {
        format!(
            "{}\n{}",
            configuration::render(input.config, input.schema),
            output.rendered_schema
        )
    } else {
        output.rendered_schema.to_string()
    };

    Ok((psl::reformat(&rendered, 2).unwrap(), output.rendered_schema.is_empty()))
}
