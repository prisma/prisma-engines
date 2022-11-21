use super::relation_names::RelationNames;
use crate::introspection_helpers::is_prisma_join_table;
use sql_schema_describer as sql;
use std::borrow::Cow;

pub(super) fn introspect_m2m_relations<'a>(relation_names: &RelationNames<'a>, ctx: &mut super::Context<'a>) {
    for table in ctx.schema.table_walkers().filter(|t| is_prisma_join_table(*t)) {
        let existing_relation = ctx.existing_m2m_relation(table.id);
        let mut fks = table.foreign_keys();

        if let (Some(first_fk), Some(second_fk)) = (fks.next(), fks.next()) {
            let (fk_a, fk_b) = if first_fk
                .constrained_columns()
                .next()
                .map(|c| c.name().eq_ignore_ascii_case("a"))
                .unwrap_or(false)
            {
                (first_fk, second_fk)
            } else {
                (second_fk, first_fk)
            };

            let [relation_name, field_a_name, field_b_name]: [Cow<'a, str>; 3] = existing_relation
                .map(|relation| {
                    let name = Cow::Owned(relation.relation_name().to_string());
                    let (field_a, field_b): (Cow<'a, str>, Cow<'a, str>) = if relation.is_self_relation() {
                        // See reasoning in the comment for the
                        // do_not_try_to_keep_custom_many_to_many_relation_names test
                        let [_, field_a, field_b] = relation_names.m2m_relation_name(table.id).to_owned();
                        (field_a, field_b)
                    } else {
                        (relation.field_a().name().into(), relation.field_b().name().into())
                    };
                    [name, field_a, field_b]
                })
                .unwrap_or_else(|| relation_names.m2m_relation_name(table.id).clone());

            calculate_many_to_many_field(fk_a, fk_b, relation_name.clone(), field_a_name, ctx);
            calculate_many_to_many_field(fk_b, fk_a, relation_name, field_b_name, ctx);
        }
    }
}

fn calculate_many_to_many_field<'a>(
    fk: sql::ForeignKeyWalker<'_>,
    other_fk: sql::ForeignKeyWalker<'_>,
    relation_name: Cow<'a, str>,
    field_name: Cow<'a, str>,
    ctx: &mut super::Context<'a>,
) {
    let opposite_model_name = ctx.table_prisma_name(other_fk.referenced_table().id).prisma_name();

    let mut field = datamodel_renderer::datamodel::ModelField::new_array(field_name, opposite_model_name);

    if !relation_name.is_empty() {
        let mut relation = datamodel_renderer::datamodel::Relation::new();
        relation.name(relation_name);
        field.relation(relation);
    }

    ctx.rendered_schema
        .model_at(ctx.target_models[&fk.referenced_table().id])
        .push_field(field);
}
