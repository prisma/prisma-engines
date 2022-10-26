use crate::{introspection::introspect, SqlFamilyTrait, SqlIntrospectionResult};
use introspection_connector::{IntrospectionContext, IntrospectionResult, Warning};
use psl::{
    builtin_connectors::*,
    datamodel_connector::Connector,
    dml::Datamodel,
    parser_database::{ast, walkers},
    Configuration,
};
use quaint::prelude::SqlFamily;
use sql_schema_describer as sql;
use std::collections::HashMap;
use tracing::debug;

pub(crate) struct CalculateDatamodelContext<'a> {
    pub(crate) config: &'a Configuration,
    pub(crate) render_config: bool,
    pub(crate) previous_datamodel: &'a Datamodel,
    pub(crate) schema: &'a sql::SqlSchema,
    pub(crate) sql_family: SqlFamily,
    pub(crate) warnings: &'a mut Vec<Warning>,
    pub(crate) previous_schema: &'a psl::ValidatedSchema,
    existing_enums: &'a HashMap<sql::EnumId, ast::EnumId>,
}

impl<'a> CalculateDatamodelContext<'a> {
    pub(crate) fn is_cockroach(&self) -> bool {
        self.active_connector().provider_name() == COCKROACH.provider_name()
    }

    pub(crate) fn foreign_keys_enabled(&self) -> bool {
        self.config
            .datasources
            .first()
            .unwrap()
            .relation_mode()
            .uses_foreign_keys()
    }

    pub(crate) fn active_connector(&self) -> &'static dyn Connector {
        self.config.datasources.first().unwrap().active_connector
    }

    /// Given a SQL enum from the database, this method returns the enum that matches it (by name)
    /// in the Prisma schema.
    pub(crate) fn existing_enum(&self, id: sql::EnumId) -> Option<walkers::EnumWalker<'a>> {
        self.existing_enums.get(&id).map(|id| self.previous_schema.db.walk(*id))
    }

    /// Given a SQL enum from the database, this method returns the name it will be given in the
    /// introspected schema. If it matches a remapped enum in the Prisma schema, it is taken into
    /// account.
    pub(crate) fn enum_prisma_name(&self, id: sql::EnumId) -> &'a str {
        self.existing_enum(id)
            .map(|enm| enm.name())
            .unwrap_or_else(|| self.schema.walk(id).name())
    }
}

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(
    schema: &sql::SqlSchema,
    ctx: &IntrospectionContext,
) -> SqlIntrospectionResult<IntrospectionResult> {
    let existing_enums = if ctx.sql_family().is_mysql() {
        schema
            .walk_columns()
            .filter_map(|col| col.column_type_family_as_enum().map(|enm| (col, enm)))
            .filter_map(|(col, sql_enum)| {
                ctx.previous_schema()
                    .db
                    .walk_models()
                    .find(|model| model.database_name() == col.table().name())
                    .and_then(|model| model.scalar_fields().find(|sf| sf.database_name() == col.name()))
                    .and_then(|scalar_field| scalar_field.field_type_as_enum())
                    .map(|ast_enum| (sql_enum.id, ast_enum.id))
            })
            // Make sure the values are the same, otherwise we're not _really_ dealing with the same
            // enum.
            .filter(|(sql_enum_id, ast_enum_id)| {
                let sql_values = schema.walk(*sql_enum_id).values();
                let prisma_values = ctx.previous_schema().db.walk(*ast_enum_id).values();
                prisma_values.len() == sql_values.len()
                    && prisma_values.zip(sql_values).all(|(a, b)| a.database_name() == b)
            })
            .collect()
    } else {
        ctx.previous_schema()
            .db
            .walk_enums()
            .filter_map(|prisma_enum| {
                schema
                    .find_enum(prisma_enum.database_name())
                    .map(|sql_id| (sql_id, prisma_enum.id))
            })
            .collect()
    };

    let mut warnings = Vec::new();

    let mut context = CalculateDatamodelContext {
        config: ctx.configuration(),
        render_config: ctx.render_config,
        previous_datamodel: &ctx.previous_data_model,
        schema,
        sql_family: ctx.sql_family(),
        previous_schema: ctx.previous_schema(),
        warnings: &mut warnings,
        existing_enums: &existing_enums,
    };

    let (version, data_model, is_empty) = introspect(&mut context)?;

    debug!("Done calculating datamodel.");

    Ok(IntrospectionResult {
        data_model,
        is_empty,
        warnings,
        version,
    })
}
