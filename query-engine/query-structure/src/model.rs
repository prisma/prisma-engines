use crate::prelude::*;
use itertools::{Either, Itertools};
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
        self.primary_identifier_scalars()
            .map(|id| self.dm.clone().zip(ScalarFieldId::InModel(id)))
            .collect_vec()
            .into()
    }

    fn primary_identifier_scalars(&self) -> impl Iterator<Item = psl::parser_database::ScalarFieldId> + use<'_> {
        if self.walker().ast_model().is_view() {
            return Either::Left(self.walker().scalar_fields().map(|sf| sf.id));
        }
        Either::Right(
            self.walker()
                .required_unique_criterias()
                .next()
                .expect("model must have at least one unique criterion")
                .fields()
                .map(|f| {
                    f.as_scalar_field()
                        .expect("primary identifier must consist of scalar fields")
                        .id
                }),
        )
    }

    pub fn shard_aware_primary_identifier(&self) -> FieldSelection {
        let id = self.primary_identifier_scalars().collect_vec();

        let sk = self
            .walker()
            .shard_key()
            .into_iter()
            .flat_map(|sk| sk.fields())
            .map(|sf| sf.id)
            .filter(|sk_field| id.iter().all(|id_field| id_field != sk_field));

        id.iter()
            .copied()
            .chain(sk)
            .map(|id| self.dm.clone().zip(ScalarFieldId::InModel(id)))
            .collect_vec()
            .into()
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
