use crate::{
    ast,
    types::IdAttribute,
    walkers::{ModelWalker, ScalarFieldAttributeWalker, ScalarFieldWalker},
    ParserDatabase,
};

#[derive(Copy, Clone)]
pub struct PrimaryKeyWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) attribute: &'db IdAttribute<'ast>,
    pub(crate) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> PrimaryKeyWalker<'ast, 'db> {
    pub fn ast_attribute(self) -> &'ast ast::Attribute {
        self.attribute.source_attribute
    }

    pub fn mapped_name(self) -> Option<&'ast str> {
        self.attribute.db_name
    }

    pub fn is_defined_on_field(self) -> bool {
        self.attribute.source_field.is_some()
    }

    pub fn model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
            model_id: self.model_id,
        }
    }

    pub fn name(self) -> Option<&'ast str> {
        self.attribute.name
    }

    pub fn fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        self.attribute.fields.iter().map(move |field| ScalarFieldWalker {
            model_id: self.model_id,
            field_id: field.field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, field.field_id)],
        })
    }

    pub fn scalar_field_attributes(self) -> impl ExactSizeIterator<Item = ScalarFieldAttributeWalker<'ast, 'db>> + 'db {
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
