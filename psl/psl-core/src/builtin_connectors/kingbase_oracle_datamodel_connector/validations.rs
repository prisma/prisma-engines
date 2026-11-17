use crate::{
    datamodel_connector::{Connector, NativeTypeInstance, walker_ext_traits::ScalarFieldWalkerExt},
    diagnostics::{DatamodelError, Diagnostics},
    parser_database::{
        SortOrder,
        walkers::{
            IndexFieldWalker, IndexWalker, ModelWalker, PrimaryKeyWalker, ScalarFieldAttributeWalker, ScalarFieldWalker,
        },
    },
};

use super::KingbaseOracleType;

const TYPES_NOT_ALLOWED_IN_KEYS: &[&str] = &["Json", "Xml"];

fn native_type_for_key_field(
    connector: &dyn Connector,
    field: ScalarFieldAttributeWalker<'_>,
) -> Option<NativeTypeInstance> {
    let field = field.as_index_field();

    field.native_type_instance(connector).or_else(|| {
        field
            .scalar_field_type()
            .is_json()
            .then(|| NativeTypeInstance::new::<KingbaseOracleType>(KingbaseOracleType::Json))
    })
}

pub(crate) fn field_types_can_be_used_in_an_index(
    connector: &dyn Connector,
    index: IndexWalker<'_>,
    errors: &mut Diagnostics,
) {
    for field in index.scalar_field_attributes() {
        let Some(native_type) = native_type_for_key_field(connector, field) else {
            continue;
        };
        let (native_type_name, _) = connector.native_type_to_parts(&native_type);

        if !TYPES_NOT_ALLOWED_IN_KEYS.contains(&native_type_name) {
            continue;
        }

        let error = if index.is_unique() {
            connector
                .native_instance_error(&native_type)
                .new_incompatible_native_type_with_unique("", index.ast_attribute().span)
        } else {
            connector
                .native_instance_error(&native_type)
                .new_incompatible_native_type_with_index("", index.ast_attribute().span)
        };

        errors.push_error(error);
        break;
    }
}

pub(crate) fn field_types_can_be_used_in_a_primary_key(
    connector: &dyn Connector,
    primary_key: PrimaryKeyWalker<'_>,
    errors: &mut Diagnostics,
) {
    for field in primary_key.scalar_field_attributes() {
        let Some(native_type) = native_type_for_key_field(connector, field) else {
            continue;
        };
        let (native_type_name, _) = connector.native_type_to_parts(&native_type);

        if !TYPES_NOT_ALLOWED_IN_KEYS.contains(&native_type_name) {
            continue;
        }

        errors.push_error(
            connector
                .native_instance_error(&native_type)
                .new_incompatible_native_type_with_id("", primary_key.ast_attribute().span),
        );
        break;
    }
}

/// R2 renders descending unique keys as standalone indexes. They enforce
/// uniqueness but cannot serve as the referenced constraint of a foreign key.
pub(crate) fn foreign_keys_must_reference_constraints(model: ModelWalker<'_>, errors: &mut Diagnostics) {
    fn same_fields<'a>(
        mut fields: impl ExactSizeIterator<Item = IndexFieldWalker<'a>>,
        references: &[ScalarFieldWalker<'a>],
    ) -> bool {
        fields.len() == references.len()
            && fields.all(|field| field.as_scalar_field().is_some_and(|field| references.contains(&field)))
    }

    if model.is_ignored() {
        return;
    }
    for relation in model
        .relation_fields()
        .filter(|relation| !relation.is_ignored() && !relation.related_model().is_ignored())
    {
        let Some(references) = relation.referenced_fields() else {
            continue;
        };
        let references: Vec<_> = references.collect();
        let target = relation.related_model();
        if target.primary_key().is_some_and(|key| {
            same_fields(
                key.scalar_field_attributes().map(|field| field.as_index_field()),
                &references,
            )
        }) {
            continue;
        }
        let mut descending_unique = false;
        let mut unique_constraint = false;
        for index in target.indexes().filter(|index| index.is_unique()) {
            if !same_fields(
                index.scalar_field_attributes().map(|field| field.as_index_field()),
                &references,
            ) {
                continue;
            }
            if index
                .scalar_field_attributes()
                .any(|field| field.sort_order() == Some(SortOrder::Desc))
            {
                descending_unique = true;
            } else {
                unique_constraint = true;
            }
        }
        if descending_unique && !unique_constraint {
            errors.push_error(DatamodelError::new_attribute_validation_error(
                "Kingbase Oracle foreign keys must reference a primary key or a unique constraint. A descending unique index alone cannot be referenced; add a non-descending unique constraint on the referenced fields or use relationMode = \"prisma\".",
                "@relation",
                relation.relation_attribute().unwrap().span,
            ));
        }
    }
}
