use crate::{
    ast,
    types::ShardKeyAttribute,
    walkers::{ModelWalker, ScalarFieldWalker},
    ParserDatabase,
};

/// A `@shardKey`/`@@shardKey` attribute in the schema.
#[derive(Copy, Clone)]
pub struct ShardKeyWalker<'db> {
    pub(crate) model_id: crate::ModelId,
    pub(crate) attribute: &'db ShardKeyAttribute,
    pub(crate) db: &'db ParserDatabase,
}

impl<'db> ShardKeyWalker<'db> {
    /// The `@(@)shardKey` AST node.
    pub fn ast_attribute(self) -> &'db ast::Attribute {
        &self.db.asts[(self.model_id.0, self.attribute.source_attribute.1)]
    }

    /// Is this a `@shardKey` on a specific field, rather than on the model?
    pub fn is_defined_on_field(self) -> bool {
        self.attribute.source_field.is_some()
    }

    /// If defined on a specific field, returns `@shardKey`. Otherwise `@@shardKey`.
    pub fn attribute_name(self) -> &'static str {
        if self.is_defined_on_field() {
            "@shardKey"
        } else {
            "@@shardKey"
        }
    }

    /// The model the shard key is defined on.
    pub fn model(self) -> ModelWalker<'db> {
        self.db.walk(self.model_id)
    }

    /// The scalar fields used as the shard key.
    pub fn fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + Clone + use<'db> {
        self.attribute.fields.iter().map(move |field| self.db.walk(*field))
    }
}
