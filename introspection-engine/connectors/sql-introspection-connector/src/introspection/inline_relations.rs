use super::{models::table_has_usable_identifier, relation_names::RelationNames};
use crate::introspection_helpers::is_prisma_join_table;
use psl::{
    datamodel_connector::{constraint_names::ConstraintNames, Connector},
    dml::{FieldArity, ReferentialAction},
};
use sql_schema_describer as sql;
use std::{borrow::Cow, collections::HashMap};

/// For each foreign key in the SQL schema, produce two relation fields in the resulting Prisma
/// schema.
pub(super) fn introspect_inline_relations(
    relation_names: &RelationNames,
    datamodel: &mut psl::dml::Datamodel,
    ctx: &mut super::Context<'_>,
) {
    let dml_model_ids: HashMap<String, usize> = datamodel
        .models
        .iter()
        .enumerate()
        .map(|(id, m)| (m.name.clone(), id))
        .collect();

    for table in ctx.schema.table_walkers().filter(|t| !is_prisma_join_table(*t)) {
        for (fk, relation_name_from_db) in table
            .foreign_keys()
            .filter_map(|fk| relation_names.inline_relation_name(fk.id).map(|name| (fk, name)))
        {
            let existing_relation = ctx.existing_inline_relation(fk.id);
            let [relation_name, forward_relation_field_name, back_relation_field_name] = existing_relation
                .and_then(|relation| {
                    Some(relation)
                        .zip(relation.forward_relation_field())
                        .zip(relation.back_relation_field())
                })
                .map(|((relation, forward), back)| {
                    [
                        relation.explicit_relation_name().unwrap_or("").into(),
                        forward.name().into(),
                        back.name().into(),
                    ]
                })
                .unwrap_or_else(|| relation_name_from_db.to_owned());

            // Forward relation field.
            {
                let referencing_model_name = ctx.table_prisma_name(table.id).prisma_name();
                let referencing_model_idx: usize = dml_model_ids[referencing_model_name.as_ref()];
                datamodel.models[referencing_model_idx].add_field(psl::dml::Field::RelationField(
                    calculate_relation_field(
                        fk,
                        (relation_name.clone().into_owned(), forward_relation_field_name.clone()),
                        ctx,
                    ),
                ));
            }

            // Back relation field.
            {
                let referenced_model_name = ctx.table_prisma_name(fk.referenced_table().id).prisma_name();
                let referenced_model_idx: usize = dml_model_ids[referenced_model_name.as_ref()];

                // Back relation field
                datamodel.models[referenced_model_idx].add_field(psl::dml::Field::RelationField(
                    calculate_backrelation_field(fk, (relation_name.into_owned(), back_relation_field_name), ctx),
                ));
            }
        }
    }
}

fn calculate_relation_field(
    foreign_key: sql::ForeignKeyWalker<'_>,
    (relation_name, field_name): (String, Cow<'_, str>),
    ctx: &mut super::Context<'_>,
) -> psl::dml::RelationField {
    let map_action = |action: sql::ForeignKeyAction| match action {
        sql::ForeignKeyAction::NoAction => ReferentialAction::NoAction,
        sql::ForeignKeyAction::Restrict => ReferentialAction::Restrict,
        sql::ForeignKeyAction::Cascade => ReferentialAction::Cascade,
        sql::ForeignKeyAction::SetNull => ReferentialAction::SetNull,
        sql::ForeignKeyAction::SetDefault => ReferentialAction::SetDefault,
    };

    let relation_info = psl::dml::RelationInfo {
        name: relation_name,
        fk_name: relation_mapped_name(foreign_key, ctx.active_connector()),
        fields: foreign_key
            .constrained_columns()
            .map(|c| ctx.column_prisma_name(c.id).prisma_name().into_owned())
            .collect(),
        referenced_model: ctx
            .table_prisma_name(foreign_key.referenced_table().id)
            .prisma_name()
            .into_owned(),
        references: foreign_key
            .referenced_columns()
            .map(|c| ctx.column_prisma_name(c.id).prisma_name().into_owned())
            .collect(),
        on_delete: None,
        on_update: None,
    };

    let arity = match foreign_key.constrained_columns().any(|c| !c.arity().is_required()) {
        true => FieldArity::Optional,
        false => FieldArity::Required,
    };

    let calculated_arity = match foreign_key.constrained_columns().any(|c| c.arity().is_required()) {
        true => FieldArity::Required,
        false => arity,
    };

    let mut relation_field = psl::dml::RelationField::new(&field_name, arity, calculated_arity, relation_info);

    let on_delete_action = map_action(foreign_key.on_delete_action());
    let on_update_action = map_action(foreign_key.on_update_action());

    relation_field.supports_restrict_action(!ctx.sql_family.is_mssql());

    // Add an @ignore attribute if 1. the parent model isn't already ignored, and 2. the referenced
    // model is ignored.
    relation_field.is_ignored = table_has_usable_identifier(foreign_key.table())
        && !table_has_usable_identifier(foreign_key.referenced_table());

    if relation_field.default_on_delete_action() != on_delete_action {
        relation_field.relation_info.on_delete = Some(on_delete_action);
    }

    if relation_field.default_on_update_action() != on_update_action {
        relation_field.relation_info.on_update = Some(on_update_action);
    }

    relation_field
}

fn calculate_backrelation_field(
    fk: sql::ForeignKeyWalker<'_>,
    (relation_name, field_name): (String, Cow<'_, str>),
    ctx: &mut super::Context<'_>,
) -> psl::dml::RelationField {
    let model_a_name = ctx.table_prisma_name(fk.table().id).prisma_name().into_owned();
    let relation_info = psl::dml::RelationInfo {
        name: relation_name,
        fk_name: None,
        referenced_model: model_a_name,
        fields: vec![],
        references: vec![],
        on_delete: None,
        on_update: None,
    };

    let forward_relation_field_is_unique = fk
        .table()
        .indexes()
        .filter(|idx| idx.is_primary_key() || idx.is_unique())
        .any(|idx| {
            idx.columns().all(|idx_col| {
                fk.constrained_columns()
                    .any(|fk_col| fk_col.id == idx_col.as_column().id)
            })
        });

    let arity = if forward_relation_field_is_unique {
        FieldArity::Optional // 1:1 relation
    } else {
        FieldArity::List
    };

    let mut field = psl::dml::RelationField::new(&field_name, arity, arity, relation_info);

    // We want to put an @ignore on the field iff the field's referenced table is ignored, and the
    // parent table isn't, because otherwise the ignore would be redundant.
    if !table_has_usable_identifier(fk.table()) && table_has_usable_identifier(fk.referenced_table()) {
        field.is_ignored = true;
    }

    field
}

fn relation_mapped_name(fk: sql::ForeignKeyWalker<'_>, connector: &dyn Connector) -> Option<String> {
    let cols: Vec<_> = fk.constrained_columns().map(|c| c.name()).collect();
    let default_name = ConstraintNames::foreign_key_constraint_name(fk.table().name(), &cols, connector);

    fk.constraint_name()
        .filter(|name| *name != default_name.as_str())
        .map(ToOwned::to_owned)
}
