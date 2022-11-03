use crate::introspection_helpers::is_prisma_join_table;
use psl::{dml, parser_database::walkers::ImplicitManyToManyRelationWalker};
use sql_schema_describer as sql;

pub(super) fn introspect_m2m_relations(datamodel: &mut dml::Datamodel, ctx: &mut super::Context<'_>) {
    for table in ctx.schema.table_walkers().filter(|t| is_prisma_join_table(*t)) {
        let existing_relation = ctx.existing_m2m_relation(table.id);
        let relation_name = existing_relation
            .and_then(|relation| match relation.relation_name() {
                psl::parser_database::walkers::RelationName::Explicit(name) => Some(name.to_owned()),
                _ => None,
            })
            .unwrap_or_else(|| table.name()[1..].to_owned());

        let mut fks = table.foreign_keys();
        if let (Some(fk_a), Some(fk_b)) = (fks.next(), fks.next()) {
            calculate_many_to_many_field(fk_a, fk_b, relation_name.clone(), existing_relation, datamodel, ctx);
            calculate_many_to_many_field(fk_b, fk_a, relation_name, existing_relation, datamodel, ctx);
        }
    }
}

fn calculate_many_to_many_field(
    fk: sql::ForeignKeyWalker<'_>,
    other_fk: sql::ForeignKeyWalker<'_>,
    relation_name: String,
    existing_relation: Option<ImplicitManyToManyRelationWalker<'_>>,
    datamodel: &mut dml::Datamodel,
    ctx: &mut super::Context<'_>,
) {
    let model = datamodel.find_model_mut(ctx.model_prisma_name(fk.referenced_table().id));
    let opposite_model_name = ctx.model_prisma_name(other_fk.referenced_table().id);
    let field_name = match fk.constrained_columns().next().map(|c| c.name()) {
        Some(other) if fk.referenced_table().id == other_fk.referenced_table().id => {
            format!("{opposite_model_name}_{other}")
        }
        Some("A") => existing_relation
            .map(|rel| rel.field_a().name())
            .unwrap_or_else(|| opposite_model_name)
            .to_owned(),
        Some("B") => existing_relation
            .map(|rel| rel.field_b().name())
            .unwrap_or_else(|| opposite_model_name)
            .to_owned(),
        _ => opposite_model_name.to_owned(),
    };

    let relation_info = dml::RelationInfo {
        name: relation_name,
        fk_name: None,
        fields: Vec::new(),
        referenced_model: opposite_model_name.to_owned(),
        references: Vec::new(),
        on_delete: None,
        on_update: None,
    };

    model.add_field(dml::Field::RelationField(dml::RelationField::new(
        &field_name,
        dml::FieldArity::List,
        dml::FieldArity::List,
        relation_info,
    )))
}
