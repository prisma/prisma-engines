use crate::{
    datamodel_connector::ConnectorCapability,
    diagnostics::DatamodelError,
    parser_database::{self, walkers::EnumWalker},
    validate::validation_pipeline::context::Context,
};

pub(super) fn schema_attribute(enm: EnumWalker<'_>, ctx: &mut Context<'_>) {
    match (enm.schema(), ctx.datasource) {
        (Some((schema_name, span)), Some(ds)) if !ds.has_schema(schema_name) => {
            ctx.push_error(DatamodelError::new_static(
                "This schema is not defined in the datasource. Read more on `@@schema` at https://pris.ly/d/multi-schema",
                span,
            ))
        },
        (Some((_, span)), _) if !ctx.connector.has_capability(ConnectorCapability::MultiSchema) => {
            ctx.push_error(DatamodelError::new_static(
                "@@schema is not supported on the current datasource provider",
                span,
            ))
        }
        (None, _) if ctx.db.schema_flags().contains(parser_database::SchemaFlags::UsesSchemaAttribute) && !ctx.connector.is_provider("mysql") => ctx.push_error(DatamodelError::new_static("This enum is missing an `@@schema` attribute.", enm.ast_enum().span)),
        _ => (),
    }
}
