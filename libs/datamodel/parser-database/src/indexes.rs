use crate::{
    context::Context,
    types::{FieldWithArgs, IndexAttribute, IndexType},
};

/// Prisma forces a 1:1 relation to be unique from the defining side. If the
/// field is not a primary key or already defined in a unique index, we add an
/// implicit unique index to that field here.
pub(super) fn infer_implicit_indexes(ctx: &mut Context<'_, '_>) {
    let mut indexes = Vec::new();

    for relation in ctx.db.walk_relations().filter_map(|rel| rel.refine().as_inline()) {
        if !relation.is_one_to_one() {
            continue;
        }

        let forward = if let Some(forward) = relation.forward_relation_field() {
            forward
        } else {
            continue;
        };

        if forward.fields().is_none() {
            continue;
        };

        let referencing_fields = || forward.fields().unwrap();

        let model = relation.referencing_model();

        if model
            .explicit_indexes()
            .filter(|index| index.is_unique())
            .any(|index| index.contains_exactly_fields(referencing_fields()))
        {
            continue;
        }

        if model
            .primary_key()
            .map(|pk| pk.contains_exactly_fields(referencing_fields()))
            .unwrap_or(false)
        {
            continue;
        }

        let source_field = {
            let mut fields = referencing_fields();

            if fields.len() == 1 {
                fields.next().map(|f| f.field_id())
            } else {
                None
            }
        };

        indexes.push((
            model.model_id(),
            IndexAttribute {
                r#type: IndexType::Unique,
                fields: referencing_fields()
                    .map(|f| FieldWithArgs {
                        field_id: f.field_id(),
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
        if let Some(model) = ctx.db.types.model_attributes.get_mut(&model_id) {
            model.implicit_indexes.push(attributes);
        }
    }
}
