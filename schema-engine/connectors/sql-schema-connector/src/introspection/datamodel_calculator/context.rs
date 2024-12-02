mod flavour;

use crate::introspection::{
    introspection_helpers::{
        is_new_migration_table, is_old_migration_table, is_prisma_m_to_n_relation, is_relay_table,
    },
    introspection_map::{IntrospectionMap, RelationName},
    introspection_pair::{EnumPair, ModelPair, RelationFieldDirection, ViewPair},
    sanitize_datamodel_names::{EnumVariantName, IntrospectedName, ModelName},
};
use psl::{
    builtin_connectors::*,
    datamodel_connector::Connector,
    parser_database::{self as db, walkers},
    Configuration, PreviewFeature,
};
use quaint::prelude::SqlFamily;
use schema_connector::IntrospectionContext;
use sql_schema_describer as sql;
use std::borrow::Cow;

pub(super) use flavour::IntrospectionFlavour;

pub(crate) struct DatamodelCalculatorContext<'a> {
    pub(crate) config: &'a Configuration,
    pub(crate) render_config: bool,
    pub(crate) sql_schema: &'a sql::SqlSchema,
    pub(crate) sql_family: SqlFamily,
    pub(crate) previous_schema: &'a psl::ValidatedSchema,
    pub(crate) introspection_map: IntrospectionMap<'a>,
    pub(crate) force_namespaces: Option<&'a [String]>,
    pub(crate) flavour: Box<dyn IntrospectionFlavour>,
    pub(crate) search_path: &'a str,
}

impl<'a> DatamodelCalculatorContext<'a> {
    pub(crate) fn new(ctx: &'a IntrospectionContext, sql_schema: &'a sql::SqlSchema, search_path: &'a str) -> Self {
        let flavour: Box<dyn IntrospectionFlavour> = match ctx.sql_family() {
            SqlFamily::Postgres => Box::new(flavour::PostgresIntrospectionFlavour),
            SqlFamily::Mysql => Box::new(flavour::MysqlIntrospectionFlavour),
            SqlFamily::Sqlite => Box::new(flavour::SqliteIntrospectionFlavour),
            SqlFamily::Mssql => Box::new(flavour::SqlServerIntrospectionFlavour),
        };

        let mut ctx = DatamodelCalculatorContext {
            config: ctx.configuration(),
            render_config: ctx.render_config,
            sql_schema,
            sql_family: ctx.sql_family(),
            previous_schema: ctx.previous_schema(),
            introspection_map: Default::default(),
            force_namespaces: ctx.namespaces(),
            flavour,
            search_path,
        };

        ctx.introspection_map = IntrospectionMap::new(&ctx);

        ctx
    }

    pub(crate) fn is_cockroach(&self) -> bool {
        self.active_connector().provider_name() == COCKROACH.provider_name()
    }

    pub(crate) fn relation_mode(&self) -> psl::datamodel_connector::RelationMode {
        self.config.datasources.first().unwrap().relation_mode()
    }

    pub(crate) fn foreign_keys_enabled(&self) -> bool {
        self.relation_mode().uses_foreign_keys()
    }

    pub(crate) fn active_connector(&self) -> &'static dyn Connector {
        self.config.datasources.first().unwrap().active_connector
    }

    pub(crate) fn uses_namespaces(&self) -> bool {
        let schemas_in_datasource = matches!(self.config.datasources.first(), Some(ds) if !ds.namespaces.is_empty());
        let schemas_in_parameters = self.force_namespaces.is_some();

        schemas_in_datasource || schemas_in_parameters
    }

    /// Iterate over the database enums, combined together with a
    /// possible existing enum in the PSL.
    pub(crate) fn enum_pairs(&'a self) -> impl Iterator<Item = EnumPair<'a>> + 'a {
        let uses_views = self.config.preview_features().contains(PreviewFeature::Views);
        let is_mysql = self.sql_family.is_mysql();

        self.sql_schema
            .enum_walkers()
            // MySQL enums are taken from the columns, which means a rogue enum might appear
            // for users not using the views preview feature, but having views with enums
            // in their database.
            .filter(move |e| !is_mysql || uses_views || self.sql_schema.enum_used_in_tables(e.id))
            .map(|next| EnumPair::new(self, self.existing_enum(next.id), next))
    }

    pub(crate) fn sql_family(&self) -> SqlFamily {
        self.sql_family
    }

    /// Iterate over the database tables, combined together with a
    /// possible existing model in the PSL.
    pub(crate) fn model_pairs(&'a self) -> impl Iterator<Item = ModelPair<'a>> + 'a {
        self.sql_schema
            .table_walkers()
            .filter(|table| !is_old_migration_table(*table))
            .filter(|table| !is_new_migration_table(*table))
            .filter(|table| !is_prisma_m_to_n_relation(*table, self.flavour.uses_pk_in_m2m_join_tables(self)))
            .filter(|table| !is_relay_table(*table))
            .map(move |next| {
                let previous = self.existing_model(next.id);
                ModelPair::new(self, previous, next)
            })
    }

    /// Iterate over the database views, combined together with a
    /// possible existing view in the PSL.
    pub(crate) fn view_pairs(&'a self) -> impl Iterator<Item = ViewPair<'a>> + 'a {
        // Right now all connectors introspect views for db reset.
        // Filtering the ones with columns will not cause
        // empty view blocks with these connectors.
        //
        // Removing the filter when all connectors are done.
        self.sql_schema
            .view_walkers()
            .filter(|v| v.columns().len() > 0)
            .map(|next| {
                let previous = self.existing_view(next.id);
                ViewPair::new(self, previous, next)
            })
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
        if let Some(r#enum) = self.existing_enum(id) {
            return ModelName::FromPsl {
                name: r#enum.name(),
                mapped_name: r#enum.mapped_name(),
            };
        }

        let r#enum = self.sql_schema.walk(id);
        ModelName::new_from_sql(r#enum.name(), r#enum.namespace(), self)
    }

    /// Given a SQL enum variant from the database catalog, this method returns the name it will be
    /// given in the introspected schema. If it matches a remapped enum value in the Prisma schema,
    /// it is taken into account.
    pub(crate) fn enum_variant_name(&self, id: sql::EnumVariantId) -> EnumVariantName<'a> {
        let variant = self.sql_schema.walk(id);
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

    pub(crate) fn existing_view(&self, id: sql::ViewId) -> Option<walkers::ModelWalker<'a>> {
        self.introspection_map
            .existing_views
            .get(&id)
            .map(|id| self.previous_schema.db.walk(*id))
    }

    pub(crate) fn existing_table_scalar_field(&self, id: sql::TableColumnId) -> Option<walkers::ScalarFieldWalker<'a>> {
        self.introspection_map
            .existing_model_scalar_fields
            .get(&id)
            .map(|id| self.previous_schema.db.walk(*id))
    }

    pub(crate) fn existing_view_scalar_field(&self, id: sql::ViewColumnId) -> Option<walkers::ScalarFieldWalker<'a>> {
        self.introspection_map
            .existing_view_scalar_fields
            .get(&id)
            .map(|id| self.previous_schema.db.walk(*id))
    }

    pub(crate) fn column_prisma_name(
        &self,
        id: sql::Either<sql::TableColumnId, sql::ViewColumnId>,
    ) -> IntrospectedName<'a> {
        match id {
            sql::Either::Left(id) => self.table_column_prisma_name(id),
            sql::Either::Right(id) => self.view_column_prisma_name(id),
        }
    }

    pub(crate) fn table_column_prisma_name(&self, id: sql::TableColumnId) -> IntrospectedName<'a> {
        self.existing_table_scalar_field(id)
            .map(|sf| IntrospectedName::FromPsl {
                name: sf.name(),
                mapped_name: sf.mapped_name(),
            })
            .unwrap_or_else(|| IntrospectedName::new_from_sql(self.sql_schema.walk(id).name()))
    }

    pub(crate) fn view_column_prisma_name(&self, id: sql::ViewColumnId) -> IntrospectedName<'a> {
        self.existing_view_scalar_field(id)
            .map(|sf| IntrospectedName::FromPsl {
                name: sf.name(),
                mapped_name: sf.mapped_name(),
            })
            .unwrap_or_else(|| IntrospectedName::new_from_sql(self.sql_schema.walk(id).name()))
    }

    // Use the existing model name when available.
    pub(crate) fn table_prisma_name(&self, id: sql::TableId) -> ModelName<'a> {
        if let Some(model) = self.existing_model(id) {
            return ModelName::FromPsl {
                name: model.name(),
                mapped_name: model.mapped_name(),
            };
        }

        let table = self.sql_schema.walk(id);
        ModelName::new_from_sql(table.name(), table.namespace(), self)
    }

    // Use the existing view name when available.
    pub(crate) fn view_prisma_name(&self, id: sql::ViewId) -> ModelName<'a> {
        if let Some(view) = self.existing_view(id) {
            return ModelName::FromPsl {
                name: view.name(),
                mapped_name: view.mapped_name(),
            };
        }

        let view = self.sql_schema.walk(id);
        ModelName::new_from_sql(view.name(), view.namespace(), self)
    }

    pub(crate) fn name_is_unique(&'a self, name: &'a str) -> bool {
        let name = crate::introspection::sanitize_datamodel_names::sanitize_string(name);

        self.introspection_map
            .top_level_names
            .get(&name)
            .map(|val| *val <= 1)
            .unwrap_or(true)
    }

    pub(crate) fn forward_inline_relation_field_prisma_name(&'a self, id: sql::ForeignKeyId) -> &'a str {
        let existing_relation = self
            .existing_inline_relation(id)
            .and_then(|relation| relation.as_complete());

        match existing_relation {
            Some(relation) => relation.referencing_field().name(),
            None => &self.inline_relation_name(id).unwrap()[1],
        }
    }

    pub(crate) fn back_inline_relation_field_prisma_name(&'a self, id: sql::ForeignKeyId) -> &'a str {
        let existing_relation = self
            .existing_inline_relation(id)
            .and_then(|relation| relation.as_complete());

        match existing_relation {
            Some(relation) => relation.referenced_field().name(),
            None => &self.inline_relation_name(id).unwrap()[2],
        }
    }

    #[track_caller]
    pub(crate) fn forward_m2m_relation_field_prisma_name(&'a self, id: sql::TableId) -> &'a str {
        let existing_relation = self.existing_m2m_relation(id);

        match existing_relation {
            Some(relation) if !relation.is_self_relation() => relation.field_a().name(),
            _ => &self.m2m_relation_name(id)[1],
        }
    }

    #[track_caller]
    pub(crate) fn back_m2m_relation_field_prisma_name(&'a self, id: sql::TableId) -> &'a str {
        let existing_relation = self.existing_m2m_relation(id);

        match existing_relation {
            Some(relation) if !relation.is_self_relation() => relation.field_b().name(),
            _ => &self.m2m_relation_name(id)[2],
        }
    }

    #[track_caller]
    pub(crate) fn inline_relation_prisma_name(&'a self, id: sql::ForeignKeyId) -> Cow<'a, str> {
        let existing_relation = self
            .existing_inline_relation(id)
            .and_then(|relation| relation.as_complete());

        match existing_relation {
            Some(relation) => match relation.referenced_field().relation_name() {
                walkers::RelationName::Explicit(name) => Cow::Borrowed(name),
                walkers::RelationName::Generated(_) => Cow::Borrowed(""),
            },
            None => Cow::Borrowed(&self.inline_relation_name(id).unwrap()[0]),
        }
    }

    #[track_caller]
    pub(crate) fn m2m_relation_prisma_name(&'a self, id: sql::TableId) -> Cow<'a, str> {
        let existing_relation = self.existing_m2m_relation(id);

        match existing_relation {
            Some(relation) => match relation.relation_name() {
                walkers::RelationName::Explicit(name) => Cow::Borrowed(name),
                walkers::RelationName::Generated(name) => Cow::Owned(name),
            },
            None => Cow::Borrowed(&self.m2m_relation_name(id)[0]),
        }
    }

    pub(crate) fn inline_relation_name(&'a self, id: sql::ForeignKeyId) -> Option<&'a RelationName<'a>> {
        self.introspection_map.relation_names.inline_relation_name(id)
    }

    #[track_caller]
    pub(crate) fn m2m_relation_name(&'a self, id: sql::TableId) -> &'a RelationName<'a> {
        self.introspection_map.relation_names.m2m_relation_name(id)
    }

    pub(crate) fn table_missing_for_model(&self, id: &db::ModelId) -> bool {
        self.introspection_map.missing_tables_for_previous_models.contains(id)
    }

    pub(crate) fn view_missing_for_model(&self, id: &db::ModelId) -> bool {
        self.introspection_map.missing_views_for_previous_models.contains(id)
    }

    pub(crate) fn inline_relations_for_table(
        &'a self,
        table_id_filter: sql::TableId,
    ) -> impl Iterator<Item = (RelationFieldDirection, sql::ForeignKeyWalker<'a>)> + 'a {
        self.introspection_map
            .inline_relation_positions
            .iter()
            .filter(move |(table_id, _, _)| *table_id == table_id_filter)
            .filter(move |(_, fk_id, _)| self.inline_relation_name(*fk_id).is_some())
            .map(|(_, fk_id, direction)| {
                let foreign_key = sql::Walker {
                    id: *fk_id,
                    schema: self.sql_schema,
                };

                (*direction, foreign_key)
            })
    }

    pub(crate) fn m2m_relations_for_table(
        &'a self,
        table_id_filter: sql::TableId,
    ) -> impl Iterator<Item = (RelationFieldDirection, sql::ForeignKeyWalker<'a>)> + 'a {
        self.introspection_map
            .m2m_relation_positions
            .iter()
            .filter(move |(table_id, _, _)| *table_id == table_id_filter)
            .map(|(_, fk_id, direction)| {
                let next = sql::Walker {
                    id: *fk_id,
                    schema: self.sql_schema,
                };

                (*direction, next)
            })
    }
}
