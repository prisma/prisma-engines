use crate::{
    ast,
    types::IdAttribute,
    walkers::{ModelWalker, ScalarFieldAttributeWalker, ScalarFieldWalker},
    ParserDatabase, ScalarFieldId,
};

/// An `@(@)id` attribute in the schema.
#[derive(Copy, Clone)]
pub struct PrimaryKeyWalker<'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) attribute: &'db IdAttribute,
    pub(crate) db: &'db ParserDatabase,
}

impl<'db> PrimaryKeyWalker<'db> {
    /// The `@(@)id` AST node.
    pub fn ast_attribute(self) -> &'db ast::Attribute {
        &self.db.ast[self.attribute.source_attribute]
    }

    /// The mapped name of the id.
    ///
    /// ```ignore
    /// @@id([a, b], map: "theName")
    ///                   ^^^^^^^^^
    /// ```
    pub fn mapped_name(self) -> Option<&'db str> {
        self.attribute.mapped_name.map(|id| &self.db[id])
    }

    /// Is this an `@id` on a specific field, rather than on the model?
    pub fn is_defined_on_field(self) -> bool {
        self.attribute.source_field.is_some()
    }

    /// If defined on a specific field, returns `@id`. Otherwise `@@id`.
    pub fn attribute_name(self) -> &'static str {
        if self.is_defined_on_field() {
            "@id"
        } else {
            "@@id"
        }
    }

    /// If true, the index defines the storage and ordering of the row. Mostly
    /// matters on SQL Server where one can change the clustering.
    pub fn clustered(self) -> Option<bool> {
        self.attribute.clustered
    }

    /// The model the id is deined on.
    pub fn model(self) -> ModelWalker<'db> {
        self.db.walk(self.model_id)
    }

    /// The `name` argument of the id attribute. The client name.
    ///
    /// ```ignore
    /// @@id([a, b], name: "theName")
    ///                    ^^^^^^^^^
    /// ```
    pub fn name(self) -> Option<&'db str> {
        self.attribute.name.map(|id| &self.db[id])
    }

    /// The scalar fields constrained by the id.
    pub fn fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + Clone + 'db {
        self.attribute
            .fields
            .iter()
            .map(move |field| self.db.walk(field.path.root()))
    }

    /// The scalar fields covered by the id, and their arguments.
    pub fn scalar_field_attributes(self) -> impl ExactSizeIterator<Item = ScalarFieldAttributeWalker<'db>> + 'db {
        self.attribute
            .fields
            .iter()
            .enumerate()
            .map(move |(field_arg_id, _)| ScalarFieldAttributeWalker {
                fields: &self.attribute.fields,
                db: self.db,
                field_arg_id,
            })
    }

    /// Do the constrained fields match exactly these?
    pub(crate) fn contains_exactly_fields_by_id(self, fields: &[ScalarFieldId]) -> bool {
        self.attribute.fields.len() == fields.len()
            && self
                .attribute
                .fields
                .iter()
                .zip(fields)
                .all(|(a, b)| matches!(a.path.field_in_index(), either::Either::Left(id)  if id == *b))
    }

    /// Do the constrained fields match exactly these?
    pub fn contains_exactly_fields(self, fields: impl ExactSizeIterator<Item = ScalarFieldWalker<'db>>) -> bool {
        self.attribute.fields.len() == fields.len() && self.fields().zip(fields).all(|(a, b)| a == b)
    }
}
