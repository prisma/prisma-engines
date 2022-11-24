use datamodel_renderer::datamodel as render;
use psl::parser_database::ast;
use sql_schema_describer as sql;
use std::{borrow::Cow, collections::HashMap};

pub(super) fn render(ctx: &mut super::Context<'_>) {
    let mut reintrospected_relations = Vec::new();
    let old_model_to_table: HashMap<ast::ModelId, sql::TableId> = ctx
        .introspection_map
        .existing_models
        .iter()
        .map(|(&table, &model)| (model, table))
        .collect();

    for relation in ctx
        .previous_schema
        .db
        .walk_relations()
        .filter_map(|r| r.refine().as_inline())
    {
        let relation_name = match relation.relation_name() {
            psl::parser_database::walkers::RelationName::Explicit(name) => name,
            psl::parser_database::walkers::RelationName::Generated(_) => "",
        };

        let fields =
            if let Some((forward, back)) = relation.forward_relation_field().zip(relation.back_relation_field()) {
                [forward, back]
            } else {
                continue;
            };

        if fields.iter().any(|f| !old_model_to_table.contains_key(&f.model().id)) {
            continue;
        }

        for rf in fields {
            let mut field = match rf.ast_field().arity {
                ast::FieldArity::Required => render::ModelField::new_required(rf.name(), rf.related_model().name()),
                ast::FieldArity::Optional => render::ModelField::new_optional(rf.name(), rf.related_model().name()),
                ast::FieldArity::List => render::ModelField::new_array(rf.name(), rf.related_model().name()),
            };

            if rf.relation_attribute().is_some() {
                let mut relation = render::Relation::new();

                if !relation_name.is_empty() {
                    relation.name(relation_name);
                }

                relation.fields(rf.fields().into_iter().flatten().map(|f| Cow::Borrowed(f.name())));
                relation.references(
                    rf.referenced_fields()
                        .into_iter()
                        .flatten()
                        .map(|f| Cow::Borrowed(f.name())),
                );

                if let Some(on_delete) = rf.explicit_on_delete() {
                    relation.on_delete(on_delete.as_str());
                }

                if let Some(on_update) = rf.explicit_on_update() {
                    relation.on_update(on_update.as_str());
                }

                field.relation(relation);
            }

            let new_model_idx = ctx.target_models[&old_model_to_table[&rf.model().model_id()]];
            ctx.rendered_schema.model_at(new_model_idx).push_field(field);

            reintrospected_relations.push(crate::warnings::Model {
                model: rf.model().name().to_owned(),
            });
        }
    }

    if !reintrospected_relations.is_empty() {
        let warning = crate::warnings::warning_relations_added_from_the_previous_data_model(&reintrospected_relations);
        ctx.warnings.push(warning);
    }
}
