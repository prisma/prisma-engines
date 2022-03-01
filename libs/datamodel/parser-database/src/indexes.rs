use crate::{
    context::Context,
    relations::{OneToOneRelationFields, RelationAttributes},
    types::{FieldWithArgs, IndexAttribute, IndexType},
};

/// Prisma forces a 1:1 relation to be unique from the defining side. If the
/// field is not a primary key or already defined in a unique index, we add an
/// implicit unique index to that field here.
pub(super) fn infer_implicit_indexes(ctx: &mut Context<'_>) {
    let mut indexes = Vec::new();

    for (model_a, _model_b, rel) in ctx
        .relations
        .iter_relations()
        .filter_map(|(model_a, model_b, relation)| match &relation.attributes {
            RelationAttributes::OneToOne(onetoone) => Some((model_a, model_b, onetoone)),
            _ => None,
        })
    {
        let forward = match rel {
            OneToOneRelationFields::Forward(fwd) => fwd,
            OneToOneRelationFields::Both(fwd, _) => fwd,
        };

        let forward = &ctx.types.relation_fields[&(model_a, *forward)];

        if forward.fields.is_none() {
            continue;
        };

        let referencing_fields = forward.fields.as_ref().unwrap();
        let model_a_attributes = &ctx.types.model_attributes[&model_a];

        if model_a_attributes
            .ast_indexes
            .iter()
            .filter(|(_, index)| index.is_unique())
            .any(|(_, index)| index.fields_match(referencing_fields))
        {
            continue;
        }

        if model_a_attributes
            .primary_key
            .as_ref()
            .map(|pk| pk.fields_match(referencing_fields))
            .unwrap_or(false)
        {
            continue;
        }

        let source_field = {
            if referencing_fields.len() == 1 {
                Some(referencing_fields[0])
            } else {
                None
            }
        };

        indexes.push((
            model_a,
            IndexAttribute {
                r#type: IndexType::Unique,
                fields: referencing_fields
                    .iter()
                    .map(|f| FieldWithArgs {
                        field_id: *f,
                        sort_order: None,
                        length: None,
                    })
                    .collect(),
                source_field,
                mapped_name: None,
                ..Default::default()
            },
        ));
    }

    for (model_id, attributes) in indexes.into_iter() {
        if let Some(model) = ctx.types.model_attributes.get_mut(&model_id) {
            model.implicit_indexes.push(attributes);
        }
    }
}
