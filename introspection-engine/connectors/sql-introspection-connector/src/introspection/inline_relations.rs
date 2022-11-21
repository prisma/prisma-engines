use super::{models::table_has_usable_identifier, relation_names::RelationNames};
use crate::introspection_helpers::is_prisma_join_table;
use datamodel_renderer::datamodel as render;
use psl::datamodel_connector::{constraint_names::ConstraintNames, Connector};
use sql_schema_describer as sql;
use std::borrow::Cow;

/// For each foreign key in the SQL catalog, produce two relation fields in the resulting Prisma
/// schema.
pub(super) fn introspect_inline_relations<'a>(relation_names: &RelationNames<'a>, ctx: &mut super::Context<'a>) {
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
                let referencing_model_idx: usize = ctx.target_models[&table.id];
                let field = calculate_relation_field(fk, (relation_name.clone(), forward_relation_field_name), ctx);
                ctx.rendered_schema.model_at(referencing_model_idx).push_field(field);
            }

            // Back relation field.
            {
                let referenced_model_idx: usize = ctx.target_models[&fk.referenced_table().id];
                let field = calculate_backrelation_field(fk, (relation_name, back_relation_field_name), ctx);
                ctx.rendered_schema.model_at(referenced_model_idx).push_field(field);
            }
        }
    }
}

fn calculate_relation_field<'a>(
    foreign_key: sql::ForeignKeyWalker<'a>,
    (relation_name, field_name): (Cow<'a, str>, Cow<'a, str>),
    ctx: &mut super::Context<'a>,
) -> render::ModelField<'a> {
    let referenced_model_name = ctx.table_prisma_name(foreign_key.referenced_table().id).prisma_name();

    let mut relation = render::Relation::new();
    let mut field = if foreign_key.constrained_columns().any(|c| !c.arity().is_required()) {
        render::ModelField::new_optional(field_name, referenced_model_name)
    } else {
        render::ModelField::new_required(field_name, referenced_model_name)
    };

    let any_field_required = foreign_key.constrained_columns().any(|c| c.arity().is_required());

    if !relation_name.is_empty() {
        relation.name(relation_name)
    }

    relation.fields(
        foreign_key
            .constrained_columns()
            .map(|col| ctx.column_prisma_name(col.id).prisma_name()),
    );

    relation.references(
        foreign_key
            .referenced_columns()
            .map(|col| ctx.column_prisma_name(col.id).prisma_name()),
    );

    match (any_field_required, foreign_key.on_delete_action()) {
        (false, sql::ForeignKeyAction::SetNull) => (),
        (true, sql::ForeignKeyAction::Restrict) => (),
        (true, sql::ForeignKeyAction::NoAction) if ctx.sql_family.is_mssql() => (),

        (_, sql::ForeignKeyAction::Cascade) => relation.on_delete("Cascade"),
        (_, sql::ForeignKeyAction::SetDefault) => relation.on_delete("SetDefault"),
        (true, sql::ForeignKeyAction::SetNull) => relation.on_delete("SetNull"),
        (_, sql::ForeignKeyAction::NoAction) => relation.on_delete("NoAction"),
        (false, sql::ForeignKeyAction::Restrict) => relation.on_delete("Restrict"),
    }

    match foreign_key.on_update_action() {
        // Cascade is the default
        sql::ForeignKeyAction::Cascade => (),
        sql::ForeignKeyAction::NoAction => relation.on_update("NoAction"),
        sql::ForeignKeyAction::Restrict => relation.on_update("Restrict"),
        sql::ForeignKeyAction::SetNull => relation.on_update("SetNull"),
        sql::ForeignKeyAction::SetDefault => relation.on_update("SetDefault"),
    }

    if let Some(mapped_name) = relation_mapped_name(foreign_key, ctx.active_connector()) {
        relation.map(mapped_name);
    }

    field.relation(relation);

    // Add an @ignore attribute if 1. the parent model isn't already ignored, and 2. the referenced
    // model is ignored.
    if table_has_usable_identifier(foreign_key.table()) && !table_has_usable_identifier(foreign_key.referenced_table())
    {
        field.ignore();
    }

    field
}

fn calculate_backrelation_field<'a>(
    fk: sql::ForeignKeyWalker<'a>,
    (relation_name, field_name): (Cow<'a, str>, Cow<'a, str>),
    ctx: &mut super::Context<'a>,
) -> render::ModelField<'a> {
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

    let model_a_name = ctx.table_prisma_name(fk.table().id).prisma_name();
    let mut field = if forward_relation_field_is_unique {
        // 1:1 relation
        render::ModelField::new_optional(field_name, model_a_name)
    } else {
        render::ModelField::new_array(field_name, model_a_name)
    };

    if !relation_name.is_empty() {
        let mut relation = render::Relation::new();
        relation.name(relation_name);
        field.relation(relation);
    }

    // We want to put an @ignore on the field iff the field's referenced table is ignored, and the
    // parent table isn't, because otherwise the ignore would be redundant.
    if !table_has_usable_identifier(fk.table()) && table_has_usable_identifier(fk.referenced_table()) {
        field.ignore();
    }

    field
}

fn relation_mapped_name<'a>(fk: sql::ForeignKeyWalker<'a>, connector: &dyn Connector) -> Option<&'a str> {
    let cols: Vec<_> = fk.constrained_columns().map(|c| c.name()).collect();
    let default_name = ConstraintNames::foreign_key_constraint_name(fk.table().name(), &cols, connector);

    fk.constraint_name().filter(|name| *name != default_name.as_str())
}
