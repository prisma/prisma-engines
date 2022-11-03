use psl::dml;

pub(super) fn reintrospect_relations(datamodel: &mut dml::Datamodel, ctx: &mut super::Context<'_>) {
    let mut reintrospected_relations = Vec::new();

    for relation in ctx
        .previous_schema
        .db
        .walk_relations()
        .filter_map(|r| r.refine().as_inline())
    {
        let relation_name = match relation.relation_name() {
            psl::parser_database::walkers::RelationName::Explicit(name) => name.to_owned(),
            psl::parser_database::walkers::RelationName::Generated(_) => String::new(),
        };

        let fields =
            if let Some((forward, back)) = relation.forward_relation_field().zip(relation.back_relation_field()) {
                [forward, back]
            } else {
                continue;
            };

        if fields.iter().any(|f| datamodel.find_model(f.model().name()).is_none()) {
            continue;
        }

        for rf in fields {
            let relation_info = dml::RelationInfo {
                referenced_model: rf.related_model().name().to_owned(),
                fields: rf
                    .referencing_fields()
                    .into_iter()
                    .flatten()
                    .map(|f| f.name().to_owned())
                    .collect(),
                references: rf
                    .referenced_fields()
                    .into_iter()
                    .flatten()
                    .map(|f| f.name().to_owned())
                    .collect(),
                name: relation_name.clone(),
                fk_name: rf.mapped_name().map(ToOwned::to_owned),
                on_delete: rf.explicit_on_delete().map(From::from),
                on_update: rf.explicit_on_update().map(From::from),
            };
            let model = datamodel.find_model_mut(rf.model().name());
            model.add_field(dml::Field::RelationField(dml::RelationField::new(
                rf.name(),
                rf.ast_field().arity.into(),
                rf.referential_arity().into(),
                relation_info,
            )));

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
