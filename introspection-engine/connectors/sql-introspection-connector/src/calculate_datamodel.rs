use crate::{
    introspection::introspect, warnings, EnumVariantName, IntrospectedName, ModelName, SqlFamilyTrait,
    SqlIntrospectionResult,
};
use introspection_connector::{IntrospectionContext, IntrospectionResult, Version, Warning};
use psl::{builtin_connectors::*, datamodel_connector::Connector, parser_database::walkers, Configuration};
use quaint::prelude::SqlFamily;
use sql_schema_describer as sql;

pub(crate) struct CalculateDatamodelContext<'a> {
    pub(crate) config: &'a Configuration,
    pub(crate) render_config: bool,
    pub(crate) schema: &'a sql::SqlSchema,
    pub(crate) sql_family: SqlFamily,
    pub(crate) warnings: &'a mut Vec<Warning>,
    pub(crate) previous_schema: &'a psl::ValidatedSchema,
    pub(crate) version: Version,
    pub(crate) prisma_1_uuid_defaults: Vec<warnings::ModelAndField>,
    pub(crate) prisma_1_cuid_defaults: Vec<warnings::ModelAndField>,
    pub(crate) fields_with_empty_names: Vec<warnings::ModelAndField>,
    pub(crate) enum_values_with_empty_names: Vec<warnings::EnumAndValue>,
    introspection_map: crate::introspection_map::IntrospectionMap,
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
        self.introspection_map
            .existing_enums
            .get(&id)
            .map(|id| self.previous_schema.db.walk(*id))
    }

    /// Given a SQL enum from the database, this method returns the name it will be given in the
    /// introspected schema. If it matches a remapped enum in the Prisma schema, it is taken into
    /// account.
    pub(crate) fn enum_prisma_name(&self, id: sql::EnumId) -> ModelName<'a> {
        self.existing_enum(id)
            .map(|enm| ModelName::FromPsl {
                name: enm.name(),
                mapped_name: enm.mapped_name(),
            })
            .unwrap_or_else(|| ModelName::new_from_sql(self.schema.walk(id).name()))
    }

    /// Given a SQL enum variant from the database catalog, this method returns the name it will be
    /// given in the introspected schema. If it matches a remapped enum value in the Prisma schema,
    /// it is taken into account.
    pub(crate) fn enum_variant_name(&self, id: sql::EnumVariantId) -> EnumVariantName<'a> {
        let variant = self.schema.walk(id);
        let variant_name = variant.name();
        self.existing_enum(variant.r#enum().id)
            .and_then(|enm| enm.values().find(|val| val.database_name() == variant_name))
            .map(|enm_value| EnumVariantName::FromPsl {
                name: enm_value.name(),
                mapped_name: enm_value.mapped_name(),
            })
            .unwrap_or_else(|| EnumVariantName::new_from_sql(variant_name))
    }

    /// Given a foreign key from the database, this methods returns the existing relation in the
    /// Prisma schema that matches it.
    pub(crate) fn existing_inline_relation(&self, id: sql::ForeignKeyId) -> Option<walkers::InlineRelationWalker<'a>> {
        self.introspection_map
            .existing_inline_relations
            .get(&id)
            .map(|relation_id| self.previous_schema.db.walk(*relation_id).refine().as_inline().unwrap())
    }

    pub(crate) fn existing_m2m_relation(
        &self,
        id: sql::TableId,
    ) -> Option<walkers::ImplicitManyToManyRelationWalker<'a>> {
        self.introspection_map
            .existing_m2m_relations
            .get(&id)
            .map(|relation_id| self.previous_schema.db.walk(*relation_id))
    }

    pub(crate) fn existing_model(&self, id: sql::TableId) -> Option<walkers::ModelWalker<'a>> {
        self.introspection_map
            .existing_models
            .get(&id)
            .map(|id| self.previous_schema.db.walk(*id))
    }

    pub(crate) fn existing_scalar_field(&self, id: sql::ColumnId) -> Option<walkers::ScalarFieldWalker<'a>> {
        self.introspection_map
            .existing_scalar_fields
            .get(&id)
            .map(|(model_id, field_id)| self.previous_schema.db.walk(*model_id).scalar_field(*field_id))
    }

    pub(crate) fn column_prisma_name(&self, id: sql::ColumnId) -> crate::IntrospectedName<'a> {
        self.existing_scalar_field(id)
            .map(|sf| IntrospectedName::FromPsl {
                name: sf.name(),
                mapped_name: sf.mapped_name(),
            })
            .unwrap_or_else(|| IntrospectedName::new_from_sql(self.schema.walk(id).name()))
    }

    // Use the existing model name when available.
    pub(crate) fn table_prisma_name(&self, id: sql::TableId) -> crate::ModelName<'a> {
        self.existing_model(id)
            .map(|model| ModelName::FromPsl {
                name: model.name(),
                mapped_name: model.mapped_name(),
            })
            // Failing that, potentially sanitize the table name.
            .unwrap_or_else(|| ModelName::new_from_sql(self.schema.walk(id).name()))
    }

    pub(crate) fn finalize_warnings(&mut self) {
        fn maybe_warn<T>(elems: &[T], warning: impl Fn(&[T]) -> Warning, warnings: &mut Vec<Warning>) {
            if !elems.is_empty() {
                warnings.push(warning(elems))
            }
        }

        maybe_warn(
            &self.prisma_1_uuid_defaults,
            warnings::warning_default_uuid_warning,
            self.warnings,
        );

        maybe_warn(
            &self.prisma_1_cuid_defaults,
            warnings::warning_default_cuid_warning,
            self.warnings,
        );

        maybe_warn(
            &self.enum_values_with_empty_names,
            warnings::warning_enum_values_with_empty_names,
            self.warnings,
        );

        maybe_warn(
            &self.fields_with_empty_names,
            warnings::warning_fields_with_empty_names,
            self.warnings,
        )
    }
}

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(
    schema: &sql::SqlSchema,
    ctx: &IntrospectionContext,
) -> SqlIntrospectionResult<IntrospectionResult> {
    let mut warnings = Vec::new();

    let mut context = CalculateDatamodelContext {
        config: ctx.configuration(),
        render_config: ctx.render_config,
        schema,
        sql_family: ctx.sql_family(),
        previous_schema: ctx.previous_schema(),
        warnings: &mut warnings,
        introspection_map: crate::introspection_map::IntrospectionMap::new(schema, ctx.previous_schema()),
        version: Version::NonPrisma,
        prisma_1_uuid_defaults: Vec::new(),
        prisma_1_cuid_defaults: Vec::new(),
        fields_with_empty_names: Vec::new(),
        enum_values_with_empty_names: Vec::new(),
    };

    context.version = crate::version_checker::check_prisma_version(&context);

    let (schema_string, is_empty) = introspect(&mut context)?;

    // Warning codes 5 and 6 are for Prisma 1 default reintrospection.
    let version = if context.warnings.iter().any(|w| ![5, 6].contains(&w.code)) {
        Version::NonPrisma
    } else {
        context.version
    };

    Ok(IntrospectionResult {
        data_model: schema_string,
        is_empty,
        version,
        warnings,
    })
}
