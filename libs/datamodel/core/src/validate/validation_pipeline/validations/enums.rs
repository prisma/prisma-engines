use crate::{
    datamodel_connector::ConnectorCapability, diagnostics::DatamodelError, parser_database::walkers::EnumWalker,
    validate::validation_pipeline::context::Context,
};

pub(super) fn schema_exists(enm: EnumWalker<'_>, ctx: &mut Context<'_>) {
    let (_schema, span) = match (enm.schema(), ctx.datasource) {
        (Some((schema_name, span)), Some(ds)) if !ds.has_schema(schema_name) => (schema_name, span),
        _ => return,
    };
    ctx.push_error(DatamodelError::new_static(
        "This schema is not defined in the datasource. Read more on `@@schema` at https://pris.ly/d/multi-schema",
        span,
    ))
}

pub(super) fn schema_capability(enm: EnumWalker<'_>, ctx: &mut Context<'_>) {
    if ctx.connector.has_capability(ConnectorCapability::MultiSchema) {
        return;
    }
    let span = if let Some((_, span)) = enm.schema() {
        span
    } else {
        return;
    };
    ctx.push_error(DatamodelError::new_static(
        "@@schema is not supported on the current datasource provider",
        span,
    ))
}
