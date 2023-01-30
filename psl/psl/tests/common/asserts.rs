use psl::parser_database::{walkers, ScalarType};

pub(crate) trait DatamodelAssert<'a> {
    fn assert_has_model(&'a self, name: &str) -> walkers::ModelWalker<'a>;
}

pub(crate) trait ModelAssert {
    fn assert_has_scalar_field(&self, t: &str) -> walkers::ScalarFieldWalker<'_>;
}

pub(crate) trait ScalarFieldAssert {
    fn assert_scalar_type(&self, t: ScalarType) -> &Self;
    fn assert_is_single_field_id(&self) -> walkers::PrimaryKeyWalker<'_>;
    fn assert_is_single_field_unique(&self) -> walkers::IndexWalker<'_>;
    fn assert_not_single_field_unique(&self) -> &Self;
}

impl<'a> DatamodelAssert<'a> for psl::ValidatedSchema {
    #[track_caller]
    fn assert_has_model(&'a self, name: &str) -> walkers::ModelWalker<'a> {
        self.db
            .walk_models()
            .find(|m| m.name() == name)
            .expect("Model {name} not found")
    }
}

impl<'a> ModelAssert for walkers::ModelWalker<'a> {
    #[track_caller]
    fn assert_has_scalar_field(&self, t: &str) -> walkers::ScalarFieldWalker<'_> {
        self.scalar_fields()
            .find(|sf| sf.name() == t)
            .expect("Could not find scalar field with name {t}")
    }
}

impl<'a> ScalarFieldAssert for walkers::ScalarFieldWalker<'a> {
    #[track_caller]
    fn assert_scalar_type(&self, t: ScalarType) -> &Self {
        assert_eq!(self.scalar_type(), Some(t));
        self
    }

    #[track_caller]
    fn assert_is_single_field_id(&self) -> walkers::PrimaryKeyWalker<'_> {
        self.model()
            .primary_key()
            .filter(|id| id.is_defined_on_field())
            .filter(|id| id.contains_exactly_fields(std::iter::once(*self)))
            .expect("Field is not a single-field id.")
    }

    #[track_caller]
    fn assert_is_single_field_unique(&self) -> walkers::IndexWalker<'_> {
        self.model()
            .indexes()
            .filter(|i| i.is_defined_on_field())
            .filter(|i| i.is_unique())
            .find(|i| i.contains_field(*self))
            .expect("Field is not a single-field unique.")
    }

    fn assert_not_single_field_unique(&self) -> &Self {
        match self
            .model()
            .indexes()
            .filter(|i| i.is_defined_on_field())
            .filter(|i| i.is_unique())
            .find(|i| i.contains_field(*self))
        {
            Some(_) => panic!("Expected field to not be part of a unique index."),
            None => self,
        }
    }
}
