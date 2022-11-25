use crate::{
    introspection::introspect,
    introspection_helpers::{is_new_migration_table, is_old_migration_table, is_prisma_join_table, is_relay_table},
    pair::{EnumPair, ModelPair, Pair},
    warnings, EnumVariantName, IntrospectedName, ModelName, SqlFamilyTrait, SqlIntrospectionResult,
};
use introspection_connector::{IntrospectionContext, IntrospectionResult, Version, Warning};
use psl::{builtin_connectors::*, datamodel_connector::Connector, parser_database::walkers, Configuration};
use quaint::prelude::SqlFamily;
use sql_schema_describer as sql;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct Warnings {
    pub(crate) warnings: Vec<Warning>,
    pub(crate) prisma_1_uuid_defaults: Vec<warnings::ModelAndField>,
    pub(crate) prisma_1_cuid_defaults: Vec<warnings::ModelAndField>,
    pub(crate) fields_with_empty_names: Vec<warnings::ModelAndField>,
    pub(crate) remapped_fields: Vec<warnings::ModelAndField>,
    pub(crate) enum_values_with_empty_names: Vec<warnings::EnumAndValue>,
    pub(crate) models_without_columns: Vec<warnings::Model>,
    pub(crate) models_without_identifiers: Vec<warnings::Model>,
    pub(crate) reintrospected_id_names: Vec<warnings::Model>,
    pub(crate) unsupported_types: Vec<warnings::ModelAndFieldAndType>,
    pub(crate) remapped_models: Vec<warnings::Model>,
}

impl Warnings {
    pub(crate) fn new() -> Self {
        Self {
            warnings: Vec::new(),
            ..Default::default()
        }
    }

    pub(crate) fn push(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    pub(crate) fn finalize(&mut self) -> Vec<Warning> {
        fn maybe_warn<T>(elems: &[T], warning: impl Fn(&[T]) -> Warning, warnings: &mut Vec<Warning>) {
            if !elems.is_empty() {
                warnings.push(warning(elems))
            }
        }

        maybe_warn(
            &self.models_without_identifiers,
            warnings::warning_models_without_identifier,
            &mut self.warnings,
        );

        maybe_warn(
            &self.unsupported_types,
            warnings::warning_unsupported_types,
            &mut self.warnings,
        );

        maybe_warn(
            &self.remapped_models,
            warnings::warning_enriched_with_map_on_model,
            &mut self.warnings,
        );

        maybe_warn(
            &self.remapped_fields,
            warnings::warning_enriched_with_map_on_field,
            &mut self.warnings,
        );

        maybe_warn(
            &self.models_without_columns,
            warnings::warning_models_without_columns,
            &mut self.warnings,
        );

        maybe_warn(
            &self.reintrospected_id_names,
            warnings::warning_enriched_with_custom_primary_key_names,
            &mut self.warnings,
        );

        maybe_warn(
            &self.prisma_1_uuid_defaults,
            warnings::warning_default_uuid_warning,
            &mut self.warnings,
        );

        maybe_warn(
            &self.prisma_1_cuid_defaults,
            warnings::warning_default_cuid_warning,
            &mut self.warnings,
        );

        maybe_warn(
            &self.enum_values_with_empty_names,
            warnings::warning_enum_values_with_empty_names,
            &mut self.warnings,
        );

        maybe_warn(
            &self.fields_with_empty_names,
            warnings::warning_fields_with_empty_names,
            &mut self.warnings,
        );

        std::mem::take(&mut self.warnings)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct InputContext<'a> {
    pub(crate) config: &'a Configuration,
    pub(crate) render_config: bool,
    pub(crate) schema: &'a sql::SqlSchema,
    pub(crate) sql_family: SqlFamily,
    pub(crate) version: Version,
    pub(crate) previous_schema: &'a psl::ValidatedSchema,
    pub(crate) introspection_map: &'a crate::introspection_map::IntrospectionMap,
}

pub(crate) struct OutputContext<'a> {
    pub(crate) rendered_schema: datamodel_renderer::Datamodel<'a>,
    pub(crate) target_models: HashMap<sql::TableId, usize>,
    pub(crate) warnings: Warnings,
}

impl<'a> OutputContext<'a> {
    pub(crate) fn finalize_warnings(&mut self) -> Vec<Warning> {
        self.warnings.finalize()
    }
}

impl<'a> InputContext<'a> {
    pub(crate) fn is_cockroach(self) -> bool {
        self.active_connector().provider_name() == COCKROACH.provider_name()
    }

    pub(crate) fn relation_mode(self) -> psl::datamodel_connector::RelationMode {
        self.config.datasources.first().unwrap().relation_mode()
    }

    pub(crate) fn foreign_keys_enabled(self) -> bool {
        self.relation_mode().uses_foreign_keys()
    }

    pub(crate) fn active_connector(self) -> &'static dyn Connector {
        self.config.datasources.first().unwrap().active_connector
    }

    /// Iterate over the database enums, combined together with a
    /// possible existing enum in the PSL.
    pub(crate) fn enum_pairs(self) -> impl ExactSizeIterator<Item = EnumPair<'a>> {
        self.schema
            .enum_walkers()
            .map(move |next| Pair::new(self, self.existing_enum(next.id), next))
    }

    /// Iterate over the database tables, combined together with a
    /// possible existing model in the PSL.
    pub(crate) fn model_pairs(self) -> impl Iterator<Item = ModelPair<'a>> {
        self.schema
            .table_walkers()
            .filter(|table| !is_old_migration_table(*table))
            .filter(|table| !is_new_migration_table(*table))
            .filter(|table| !is_prisma_join_table(*table))
            .filter(|table| !is_relay_table(*table))
            .map(move |next| {
                let previous = self.existing_model(next.id);
                Pair::new(self, previous, next)
            })
    }

    /// Given a SQL enum from the database, this method returns the enum that matches it (by name)
    /// in the Prisma schema.
    pub(crate) fn existing_enum(self, id: sql::EnumId) -> Option<walkers::EnumWalker<'a>> {
        self.introspection_map
            .existing_enums
            .get(&id)
            .map(|id| self.previous_schema.db.walk(*id))
    }

    /// Given a SQL enum from the database, this method returns the name it will be given in the
    /// introspected schema. If it matches a remapped enum in the Prisma schema, it is taken into
    /// account.
    pub(crate) fn enum_prisma_name(self, id: sql::EnumId) -> ModelName<'a> {
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
    pub(crate) fn enum_variant_name(self, id: sql::EnumVariantId) -> EnumVariantName<'a> {
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
    pub(crate) fn existing_inline_relation(self, id: sql::ForeignKeyId) -> Option<walkers::InlineRelationWalker<'a>> {
        self.introspection_map
            .existing_inline_relations
            .get(&id)
            .map(|relation_id| self.previous_schema.db.walk(*relation_id).refine().as_inline().unwrap())
    }

    pub(crate) fn existing_m2m_relation(
        self,
        id: sql::TableId,
    ) -> Option<walkers::ImplicitManyToManyRelationWalker<'a>> {
        self.introspection_map
            .existing_m2m_relations
            .get(&id)
            .map(|relation_id| self.previous_schema.db.walk(*relation_id))
    }

    pub(crate) fn existing_model(self, id: sql::TableId) -> Option<walkers::ModelWalker<'a>> {
        self.introspection_map
            .existing_models
            .get(&id)
            .map(|id| self.previous_schema.db.walk(*id))
    }

    pub(crate) fn existing_scalar_field(self, id: sql::ColumnId) -> Option<walkers::ScalarFieldWalker<'a>> {
        self.introspection_map
            .existing_scalar_fields
            .get(&id)
            .map(|(model_id, field_id)| self.previous_schema.db.walk(*model_id).scalar_field(*field_id))
    }

    pub(crate) fn column_prisma_name(self, id: sql::ColumnId) -> crate::IntrospectedName<'a> {
        self.existing_scalar_field(id)
            .map(|sf| IntrospectedName::FromPsl {
                name: sf.name(),
                mapped_name: sf.mapped_name(),
            })
            .unwrap_or_else(|| IntrospectedName::new_from_sql(self.schema.walk(id).name()))
    }

    // Use the existing model name when available.
    pub(crate) fn table_prisma_name(self, id: sql::TableId) -> crate::ModelName<'a> {
        self.existing_model(id)
            .map(|model| ModelName::FromPsl {
                name: model.name(),
                mapped_name: model.mapped_name(),
            })
            // Failing that, potentially sanitize the table name.
            .unwrap_or_else(|| ModelName::new_from_sql(self.schema.walk(id).name()))
    }
}

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(
    schema: &sql::SqlSchema,
    ctx: &IntrospectionContext,
) -> SqlIntrospectionResult<IntrospectionResult> {
    let introspection_map = crate::introspection_map::IntrospectionMap::new(schema, ctx.previous_schema());

    let mut input = InputContext {
        version: Version::NonPrisma,
        config: ctx.configuration(),
        render_config: ctx.render_config,
        schema,
        sql_family: ctx.sql_family(),
        previous_schema: ctx.previous_schema(),
        introspection_map: &introspection_map,
    };

    let mut output = OutputContext {
        rendered_schema: datamodel_renderer::Datamodel::default(),
        target_models: HashMap::default(),
        warnings: Warnings::new(),
    };

    input.version = crate::version_checker::check_prisma_version(&input);

    let (schema_string, is_empty) = introspect(input, &mut output)?;
    let warnings = output.finalize_warnings();

    // Warning codes 5 and 6 are for Prisma 1 default reintrospection.
    let version = if warnings.iter().any(|w| ![5, 6].contains(&w.code)) {
        Version::NonPrisma
    } else {
        input.version
    };

    Ok(IntrospectionResult {
        data_model: schema_string,
        is_empty,
        version,
        warnings,
    })
}
