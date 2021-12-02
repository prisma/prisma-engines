use crate::{
    ast,
    common::constraint_names::ConstraintNames,
    transform::ast_to_dml::db::{
        types::IdAttribute,
        walkers::{ScalarFieldAttributeWalker, ScalarFieldWalker},
        ParserDatabase,
    },
    SortOrder,
};
use datamodel_connector::Connector;
use std::borrow::Cow;

#[derive(Copy, Clone)]
pub(crate) struct PrimaryKeyWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) attribute: &'db IdAttribute<'ast>,
    pub(crate) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> PrimaryKeyWalker<'ast, 'db> {
    #[track_caller]
    pub(crate) fn ast_attribute(self) -> &'ast ast::Attribute {
        self.attribute.source_attribute
    }

    pub(crate) fn db_name(self) -> Option<&'ast str> {
        self.attribute.db_name
    }

    pub(crate) fn final_database_name(self, connector: &dyn Connector) -> Option<Cow<'ast, str>> {
        if !connector.supports_named_primary_keys() {
            return None;
        }

        Some(self.attribute.db_name.map(Cow::Borrowed).unwrap_or_else(|| {
            ConstraintNames::primary_key_name(self.db.walk_model(self.model_id).final_database_name(), connector).into()
        }))
    }

    pub(crate) fn is_defined_on_field(self) -> bool {
        self.attribute.source_field.is_some()
    }

    pub(crate) fn iter_ast_fields(
        self,
    ) -> impl Iterator<Item = (&'ast ast::Field, Option<SortOrder>, Option<u32>)> + 'db {
        self.attribute.fields.iter().map(move |field| {
            (
                &self.db.ast[self.model_id][field.field_id],
                field.sort_order,
                field.length,
            )
        })
    }

    pub(crate) fn name(self) -> Option<&'ast str> {
        self.attribute.name
    }

    pub(crate) fn fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        self.attribute.fields.iter().map(move |field| ScalarFieldWalker {
            model_id: self.model_id,
            field_id: field.field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, field.field_id)],
        })
    }

    pub(crate) fn scalar_field_attributes(
        self,
    ) -> impl ExactSizeIterator<Item = ScalarFieldAttributeWalker<'ast, 'db>> + 'db {
        self.attribute
            .fields
            .iter()
            .enumerate()
            .map(move |(field_arg_id, _)| ScalarFieldAttributeWalker {
                model_id: self.model_id,
                fields: &self.attribute.fields,
                db: self.db,
                field_arg_id,
            })
    }

    pub(crate) fn contains_exactly_fields_by_id(self, fields: &[ast::FieldId]) -> bool {
        self.attribute.fields.len() == fields.len()
            && self.attribute.fields.iter().zip(fields).all(|(a, b)| a.field_id == *b)
    }

    pub(crate) fn contains_exactly_fields(
        self,
        fields: impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>>,
    ) -> bool {
        self.attribute.fields.len() == fields.len() && self.fields().zip(fields).all(|(a, b)| a == b)
    }
}
