use crate::introspection_helpers::is_prisma_join_table;
use psl::{
    datamodel_connector::{constraint_names::ConstraintNames, Connector},
    dml::{FieldArity, ReferentialAction},
    parser_database::walkers,
};
use sql_schema_describer as sql;
use std::collections::{HashMap, HashSet};

/// For each foreign key in the SQL schema, produce two relation fields in the resulting Prisma
/// schema.
pub(super) fn introspect_inline_relations(datamodel: &mut psl::dml::Datamodel, ctx: &mut super::Context<'_>) {
    let mut duplicated_foreign_keys = Default::default();
    let m2m_table_names: HashSet<String> = ctx
        .schema
        .table_walkers()
        .filter(|table| is_prisma_join_table(*table))
        .map(|table| table.name()[1..].to_string())
        .collect();

    let dml_model_ids: HashMap<String, usize> = datamodel
        .models
        .iter()
        .enumerate()
        .map(|(id, m)| (m.name.clone(), id))
        .collect();

    for table in ctx.schema.table_walkers().filter(|t| !is_prisma_join_table(*t)) {
        collect_duplicated_fks(table, &mut duplicated_foreign_keys);

        for fk in table
            .foreign_keys()
            .filter(|fk| !duplicated_foreign_keys.contains(&fk.id))
        {
            let existing_relation = ctx.existing_inline_relation(fk.id);
            let relation_name: String = existing_relation
                .map(|relation| relation.relation_name().to_string())
                .unwrap_or_else(|| calculate_relation_name(fk, &m2m_table_names, &duplicated_foreign_keys));
            let field_names_clash_with_m2m_relation_field_names = existing_relation.is_some() // short-circuit
                || ctx
                    .schema
                    .table_walkers()
                    .filter(|table| is_prisma_join_table(*table))
                    .any(|table| {
                        table
                            .foreign_keys()
                            .any(|m2m_fk| m2m_fk.referenced_table().id == fk.table().id)
                            && table
                                .foreign_keys()
                                .any(|m2m_fk| m2m_fk.referenced_table().id == fk.referenced_table().id)
                    });

            // Forward relation field.
            {
                let referencing_model_name: &str = ctx.model_prisma_name(table.id);
                let referencing_model_idx: usize = dml_model_ids[referencing_model_name];
                datamodel.models[referencing_model_idx].add_field(psl::dml::Field::RelationField(
                    calculate_relation_field(
                        fk,
                        &relation_name,
                        existing_relation,
                        field_names_clash_with_m2m_relation_field_names,
                        ctx,
                    ),
                ));
            }

            // Back relation field.
            {
                let referenced_model_name: &str = ctx.model_prisma_name(fk.referenced_table().id);
                let referenced_model_idx: usize = dml_model_ids[referenced_model_name];

                // Back relation field
                datamodel.models[referenced_model_idx].add_field(psl::dml::Field::RelationField(
                    calculate_backrelation_field(
                        fk,
                        relation_name,
                        existing_relation,
                        field_names_clash_with_m2m_relation_field_names,
                        ctx,
                    ),
                ));
            }
        }
    }
}

fn calculate_relation_field(
    foreign_key: sql::ForeignKeyWalker<'_>,
    relation_name: &String,
    existing_relation: Option<walkers::InlineRelationWalker<'_>>,
    field_names_clash_with_m2m_relation_field_names: bool,
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
        name: relation_name.clone(),
        fk_name: relation_mapped_name(foreign_key, ctx.active_connector()),
        fields: foreign_key
            .constrained_columns()
            .map(|c| ctx.column_prisma_name(c.id).to_owned())
            .collect(),
        referenced_model: ctx.model_prisma_name(foreign_key.referenced_table().id).to_owned(),
        references: foreign_key
            .referenced_columns()
            .map(|c| ctx.column_prisma_name(c.id).to_owned())
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

    let field_name = forward_relation_field_name(
        existing_relation,
        foreign_key,
        field_names_clash_with_m2m_relation_field_names,
        relation_name,
        ctx,
    );
    let mut relation_field = psl::dml::RelationField::new(&field_name, arity, calculated_arity, relation_info);

    let on_delete_action = map_action(foreign_key.on_delete_action());
    let on_update_action = map_action(foreign_key.on_update_action());

    relation_field.supports_restrict_action(!ctx.sql_family.is_mssql());

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
    relation_name: String,
    existing_relation: Option<walkers::InlineRelationWalker<'_>>,
    field_names_clash_with_m2m_relation_field_names: bool,
    ctx: &mut super::Context<'_>,
) -> psl::dml::RelationField {
    let model_a_name = ctx.model_prisma_name(fk.table().id).to_owned();
    let relation_info = psl::dml::RelationInfo {
        name: relation_name.clone(),
        fk_name: None,
        referenced_model: model_a_name.clone(),
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

    let field_name = if fk.is_self_relation() && existing_relation.is_none() {
        format!("other_{model_a_name}") // we need to generate a different name for the backrelation field
    } else {
        existing_relation
            .and_then(|relation| relation.back_relation_field().map(|field| field.name().to_owned()))
            .unwrap_or_else(|| {
                let mut name = model_a_name.to_owned();
                if field_names_clash_with_m2m_relation_field_names {
                    name.push_str(&relation_name);
                }
                name
            })
    };

    psl::dml::RelationField::new(&field_name, arity, arity, relation_info)
}

/// This is not used for Prisma many to many relations. For them the name is the name of the join
/// table.
fn calculate_relation_name(
    fk: sql::ForeignKeyWalker<'_>,
    m2m_table_names: &HashSet<String>,
    duplicated_foreign_keys: &HashSet<sql::ForeignKeyId>,
) -> String {
    let referenced_model = fk.referenced_table().name();
    let model_with_fk = fk.table().name();
    let fk_column_name = fk.constrained_columns().map(|c| c.name()).collect::<Vec<_>>().join("_");
    let unambiguous_name = psl::RelationNames::name_for_unambiguous_relation(model_with_fk, referenced_model);

    // this needs to know whether there are m2m relations and then use ambiguous name path
    if default_relation_name_is_ambiguous(fk, duplicated_foreign_keys) || m2m_table_names.contains(&unambiguous_name) {
        psl::RelationNames::name_for_ambiguous_relation(model_with_fk, referenced_model, &fk_column_name)
    } else {
        unambiguous_name
    }
}

fn default_relation_name_is_ambiguous(
    fk: sql::ForeignKeyWalker<'_>,
    duplicated_foreign_keys: &HashSet<sql::ForeignKeyId>,
) -> bool {
    let mut both_ids = [fk.referenced_table().id, fk.table().id];
    both_ids.sort();
    fk.schema.walk_foreign_keys().any(|other_fk| {
        let mut other_ids = [other_fk.referenced_table().id, other_fk.table().id];
        other_ids.sort();

        other_fk.id != fk.id && both_ids == other_ids && !duplicated_foreign_keys.contains(&other_fk.id)
    })
}

pub(crate) fn relation_mapped_name(fk: sql::ForeignKeyWalker<'_>, connector: &dyn Connector) -> Option<String> {
    let cols: Vec<_> = fk.constrained_columns().map(|c| c.name()).collect();
    let default_name = ConstraintNames::foreign_key_constraint_name(fk.table().name(), &cols, connector);

    fk.constraint_name()
        .filter(|name| *name != default_name.as_str())
        .map(ToOwned::to_owned)
}

fn collect_duplicated_fks(table: sql::TableWalker<'_>, fks: &mut HashSet<sql::ForeignKeyId>) {
    let new_fks = table
        .foreign_keys()
        .enumerate()
        .filter(|(idx, left)| {
            let mut already_visited = table.foreign_keys().take(*idx);
            already_visited.any(|right| {
                let (left_constrained, right_constrained) = (left.constrained_columns(), right.constrained_columns());
                left_constrained.len() == right_constrained.len()
                    && left_constrained
                        .zip(right_constrained)
                        .all(|(left, right)| left.id == right.id)
                    && left
                        .referenced_columns()
                        .zip(right.referenced_columns())
                        .all(|(left, right)| left.id == right.id)
            })
        })
        .map(|(_, fk)| fk.id);
    fks.clear();
    fks.extend(new_fks)
}

fn forward_relation_field_name(
    existing_relation: Option<walkers::InlineRelationWalker<'_>>,
    foreign_key: sql::ForeignKeyWalker<'_>,
    field_names_clash_with_m2m_relation_field_names: bool,
    relation_name: &str,
    ctx: &mut super::Context<'_>,
) -> String {
    existing_relation
        .and_then(|r| r.forward_relation_field())
        .map(|f| f.name().to_owned())
        .unwrap_or_else(|| {
            let mut name = ctx.model_prisma_name(foreign_key.referenced_table().id).to_owned();
            if field_names_clash_with_m2m_relation_field_names {
                name.push_str(relation_name);
            }
            name
        })
}
