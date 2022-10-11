use crate::{
    datamodel_connector::ConnectorCapability,
    diagnostics::DatamodelError,
    parser_database::{self, ast::WithSpan, walkers::EnumWalker},
    validate::validation_pipeline::context::Context,
};
use std::collections::HashSet;

pub(super) fn database_name_clashes(ctx: &mut Context<'_>) {
    let mut database_names: HashSet<(Option<&str>, &str)> = HashSet::with_capacity(ctx.db.enums_count());

    for enm in ctx.db.walk_enums() {
        if !database_names.insert((enm.schema().map(|(n, _)| n), enm.database_name())) {
            ctx.push_error(DatamodelError::new_duplicate_enum_database_name_error(
                enm.ast_enum().span(),
            ));
        }
    }
}

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
