use crate::prelude::*;
use psl::parser_database::{walkers, ModelId};

pub type Model = crate::Zipper<ModelId>;

impl Model {
    pub fn name(&self) -> &str {
        self.walker().name()
    }

    /// Returns the set of fields to be used as the primary identifier for a record of that model.
    /// The identifier is nothing but an internal convention to have an anchor point for querying, or in other words,
    /// the identifier is not to be mistaken for a stable, external identifier, but has to be understood as
    /// implementation detail that is used to reason over a fixed set of fields.
    pub fn primary_identifier(&self) -> FieldSelection {
        let fields: Vec<_> = self
            .walker()
            .required_unique_criterias()
            .next()
            .unwrap()
            .fields()
            .map(|f| {
                self.dm
                    .clone()
                    .zip(ScalarFieldId::InModel(f.as_scalar_field().unwrap().id))
            })
            .collect();

        FieldSelection::from(fields)
    }

    pub fn fields(&self) -> Fields<'_> {
        Fields::new(self)
    }

    pub fn supports_create_operation(&self) -> bool {
        let has_unsupported_field = self
            .walker()
            .scalar_fields()
            .any(|sf| sf.ast_field().arity.is_required() && sf.is_unsupported() && sf.default_value().is_none());

        !has_unsupported_field
    }

    /// The name of the model in the database
    /// For a sql database this will be the Table name for this model
    pub fn db_name(&self) -> &str {
        self.walker().database_name()
    }

    pub fn db_name_opt(&self) -> Option<&str> {
        self.walker().mapped_name()
    }

    pub fn unique_indexes(&self) -> impl Iterator<Item = walkers::IndexWalker<'_>> {
        self.walker()
            .indexes()
            .filter(|idx| idx.is_unique())
            .filter(|index| !index.fields().any(|f| f.is_unsupported()))
    }
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Model").field(&self.name()).finish()
    }
}
