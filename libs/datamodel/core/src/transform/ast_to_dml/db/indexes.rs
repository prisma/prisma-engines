use std::borrow::Cow;

use crate::common::constraint_names::ConstraintNames;

use super::{context::Context, types::IndexAttribute};

/// Prisma forces a 1:1 relation to be unique from the defining side. If the
/// field is not a primary key or already defined in a unique index, we add an
/// implicit unique index to that field here.
pub(super) fn infer_implicit_indexes(ctx: &mut Context<'_>) {
    let mut indexes = Vec::new();

    for relation in ctx.db.walk_explicit_relations() {
        if !relation.relation_type().is_one_to_one() {
            continue;
        }

        let model = relation.referencing_model();

        if model
            .explicit_indexes()
            .filter(|index| index.is_unique())
            .any(|index| index.contains_exactly_fields(relation.referencing_fields()))
        {
            continue;
        }

        if model
            .primary_key()
            .map(|pk| pk.contains_exactly_fields(relation.referencing_fields()))
            .unwrap_or(false)
        {
            continue;
        }

        let column_names = relation
            .referencing_fields()
            .map(|f| f.final_database_name())
            .collect::<Vec<_>>();

        let db_name =
            ConstraintNames::unique_index_name(model.final_database_name(), &column_names, ctx.db.active_connector());

        let source_field = {
            let mut fields = relation.referencing_fields();

            if fields.len() == 1 {
                fields.next().map(|f| f.field_id())
            } else {
                None
            }
        };

        indexes.push((
            model.model_id(),
            IndexAttribute {
                is_unique: true,
                fields: relation.referencing_fields().map(|f| f.field_id()).collect(),
                source_field,
                name: None,
                db_name: Some(Cow::from(db_name)),
            },
        ));
    }

    for (model_id, attributes) in indexes.into_iter() {
        if let Some(model) = ctx.db.types.model_attributes.get_mut(&model_id) {
            model.implicit_indexes.push(attributes);
        }
    }
}
