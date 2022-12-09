//! Tooling to go from PSL and database schema to a PSL string.

mod configuration;
mod defaults;
mod enums;
mod indexes;
mod models;
mod postgres;
mod relation_field;
mod scalar_field;

use crate::datamodel_calculator::{InputContext, OutputContext};
pub(crate) use crate::SqlError;

/// Combines the SQL database schema and an existing PSL schema to a
/// PSL schema definition string.
pub(crate) fn to_psl_string<'a>(
    input: InputContext<'a>,
    output: &mut OutputContext<'a>,
) -> Result<(String, bool), SqlError> {
    enums::render(input, output);
    models::render(input, output);

    let psl_string = if input.render_config {
        format!(
            "{}\n{}",
            configuration::render(input.config, input.schema),
            output.rendered_schema
        )
    } else {
        output.rendered_schema.to_string()
    };

    Ok((
        psl::reformat(&psl_string, 2).unwrap(),
        output.rendered_schema.is_empty(),
    ))
}
