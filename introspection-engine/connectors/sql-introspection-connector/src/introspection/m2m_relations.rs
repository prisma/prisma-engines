use super::relation_names::RelationNames;
use crate::introspection_helpers::is_prisma_join_table;
use psl::dml;
use sql_schema_describer as sql;
use std::borrow::Cow;

pub(super) fn introspect_m2m_relations(
    relation_names: &RelationNames<'_>,
    datamodel: &mut dml::Datamodel,
    ctx: &mut super::Context<'_>,
) {
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

            let [relation_name, field_a_name, field_b_name] = existing_relation
                .map(|relation| {
                    let name = Cow::Owned(relation.relation_name().to_string());
                    let (field_a, field_b): (Cow<'_, str>, Cow<'_, str>) = if relation.is_self_relation() {
                        // See reasoning in the comment for the
                        // do_not_try_to_keep_custom_many_to_many_relation_names test
                        let [_, field_a, field_b] = relation_names.m2m_relation_name(table.id);
                        (Cow::Borrowed(field_a), Cow::Borrowed(field_b))
                    } else {
                        (relation.field_a().name().into(), relation.field_b().name().into())
                    };
                    [name, field_a, field_b]
                })
                .unwrap_or_else(|| relation_names.m2m_relation_name(table.id).clone());

            calculate_many_to_many_field(fk_a, fk_b, &relation_name, &field_a_name, datamodel, ctx);
            calculate_many_to_many_field(fk_b, fk_a, &relation_name, &field_b_name, datamodel, ctx);
        }
    }
}

fn calculate_many_to_many_field(
    fk: sql::ForeignKeyWalker<'_>,
    other_fk: sql::ForeignKeyWalker<'_>,
    relation_name: &str,
    field_name: &str,
    datamodel: &mut dml::Datamodel,
    ctx: &mut super::Context<'_>,
) {
    let model = datamodel.find_model_mut(&ctx.table_prisma_name(fk.referenced_table().id).prisma_name());
    let opposite_model_name = ctx.table_prisma_name(other_fk.referenced_table().id).prisma_name();

    let relation_info = dml::RelationInfo {
        name: relation_name.to_owned(),
        fk_name: None,
        fields: Vec::new(),
        referenced_model: opposite_model_name.clone().into_owned(),
        references: Vec::new(),
        on_delete: None,
        on_update: None,
    };

    model.add_field(dml::Field::RelationField(dml::RelationField::new(
        field_name,
        dml::FieldArity::List,
        dml::FieldArity::List,
        relation_info,
    )))
}
