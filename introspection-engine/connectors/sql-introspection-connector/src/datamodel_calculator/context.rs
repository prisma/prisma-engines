use crate::{
    introspection_helpers::{is_new_migration_table, is_old_migration_table, is_prisma_join_table, is_relay_table},
    introspection_map::RelationName,
    pair::{EnumPair, ModelPair, Pair, RelationFieldDirection},
    warnings, EnumVariantName, IntrospectedName, ModelName,
};
use introspection_connector::{Version, Warning};
use psl::{
    builtin_connectors::*,
    datamodel_connector::Connector,
    parser_database::{ast, walkers},
    Configuration,
};
use quaint::prelude::SqlFamily;
use sql_schema_describer as sql;
use std::borrow::Cow;

pub(crate) struct OutputContext<'a> {
    pub(crate) rendered_schema: datamodel_renderer::Datamodel<'a>,
    pub(crate) warnings: warnings::Warnings,
}

impl<'a> OutputContext<'a> {
    pub(crate) fn finalize_warnings(&mut self) -> Vec<Warning> {
        self.warnings.finalize()
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
    pub(crate) introspection_map: &'a crate::introspection_map::IntrospectionMap<'a>,
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

    pub(crate) fn forward_inline_relation_field_prisma_name(self, id: sql::ForeignKeyId) -> &'a str {
        let existing_relation = self
            .existing_inline_relation(id)
            .and_then(|relation| relation.as_complete());

        match existing_relation {
            Some(relation) => relation.referencing_field().name(),
            None => &self.inline_relation_name(id).unwrap()[1],
        }
    }

    pub(crate) fn back_inline_relation_field_prisma_name(self, id: sql::ForeignKeyId) -> &'a str {
        let existing_relation = self
            .existing_inline_relation(id)
            .and_then(|relation| relation.as_complete());

        match existing_relation {
            Some(relation) => relation.referenced_field().name(),
            None => &self.inline_relation_name(id).unwrap()[2],
        }
    }

    #[track_caller]
    pub(crate) fn forward_m2m_relation_field_prisma_name(self, id: sql::TableId) -> &'a str {
        let existing_relation = self.existing_m2m_relation(id);

        match existing_relation {
            Some(relation) if !relation.is_self_relation() => relation.field_a().name(),
            _ => &self.m2m_relation_name(id)[1],
        }
    }

    #[track_caller]
    pub(crate) fn back_m2m_relation_field_prisma_name(self, id: sql::TableId) -> &'a str {
        let existing_relation = self.existing_m2m_relation(id);

        match existing_relation {
            Some(relation) if !relation.is_self_relation() => relation.field_b().name(),
            _ => &self.m2m_relation_name(id)[2],
        }
    }

    #[track_caller]
    pub(crate) fn inline_relation_prisma_name(self, id: sql::ForeignKeyId) -> Cow<'a, str> {
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
    pub(crate) fn m2m_relation_prisma_name(self, id: sql::TableId) -> Cow<'a, str> {
        let existing_relation = self.existing_m2m_relation(id);

        match existing_relation {
            Some(relation) => match relation.relation_name() {
                walkers::RelationName::Explicit(name) => Cow::Borrowed(name),
                walkers::RelationName::Generated(name) => Cow::Owned(name),
            },
            None => Cow::Borrowed(&self.m2m_relation_name(id)[0]),
        }
    }

    pub(crate) fn inline_relation_name(self, id: sql::ForeignKeyId) -> Option<&'a RelationName<'a>> {
        self.introspection_map.relation_names.inline_relation_name(id)
    }

    #[track_caller]
    pub(crate) fn m2m_relation_name(self, id: sql::TableId) -> &'a RelationName<'a> {
        self.introspection_map.relation_names.m2m_relation_name(id)
    }

    pub(crate) fn table_missing_for_model(self, id: &ast::ModelId) -> bool {
        self.introspection_map.missing_tables_for_previous_models.contains(id)
    }

    pub(crate) fn inline_relations_for_table(
        self,
        table_id_filter: sql::TableId,
    ) -> impl Iterator<Item = (RelationFieldDirection, sql::ForeignKeyWalker<'a>)> {
        self.introspection_map
            .inline_relation_positions
            .iter()
            .filter(move |(table_id, _, _)| *table_id == table_id_filter)
            .filter(move |(_, fk_id, _)| self.inline_relation_name(*fk_id).is_some())
            .map(|(_, fk_id, direction)| {
                let foreign_key = sql::Walker {
                    id: *fk_id,
                    schema: self.schema,
                };

                (*direction, foreign_key)
            })
    }

    pub(crate) fn m2m_relations_for_table(
        self,
        table_id_filter: sql::TableId,
    ) -> impl Iterator<Item = (RelationFieldDirection, sql::ForeignKeyWalker<'a>)> {
        self.introspection_map
            .m2m_relation_positions
            .iter()
            .filter(move |(table_id, _, _)| *table_id == table_id_filter)
            .map(|(_, fk_id, direction)| {
                let next = sql::Walker {
                    id: *fk_id,
                    schema: self.schema,
                };

                (*direction, next)
            })
    }
}
