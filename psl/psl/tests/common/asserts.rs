use dml::ReferentialAction;
use psl::parser_database::{walkers, ScalarType};

pub(crate) trait DatamodelAssert<'a> {
    fn assert_has_model(&'a self, name: &str) -> walkers::ModelWalker<'a>;
}

pub(crate) trait ModelAssert<'a> {
    fn assert_field_count(self, count: usize) -> Self;
    fn assert_has_scalar_field(self, t: &str) -> walkers::ScalarFieldWalker<'a>;
    fn assert_has_relation_field(self, name: &str) -> walkers::RelationFieldWalker<'a>;
    fn assert_ignored(self, ignored: bool) -> Self;
}

pub(crate) trait ScalarFieldAssert {
    fn assert_scalar_type(&self, t: ScalarType) -> &Self;
    fn assert_is_single_field_id(&self) -> walkers::PrimaryKeyWalker<'_>;
    fn assert_is_single_field_unique(&self) -> walkers::IndexWalker<'_>;
    fn assert_not_single_field_unique(&self) -> &Self;
    fn assert_ignored(self, ignored: bool) -> Self;
}

pub(crate) trait RelationFieldAssert {
    fn assert_ignored(self, ignored: bool) -> Self;
    fn assert_relation_to(self, model_id: psl::schema_ast::ast::ModelId) -> Self;
    fn assert_relation_delete_strategy(self, action: ReferentialAction) -> Self;
    fn assert_relation_update_strategy(self, action: ReferentialAction) -> Self;
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

impl<'a> RelationFieldAssert for walkers::RelationFieldWalker<'a> {
    fn assert_relation_to(self, model_id: psl::schema_ast::ast::ModelId) -> Self {
        assert!(self.references_model(model_id));
        self
    }

    fn assert_ignored(self, ignored: bool) -> Self {
        assert_eq!(self.is_ignored(), ignored);
        self
    }

    fn assert_relation_delete_strategy(self, action: ReferentialAction) -> Self {
        assert_eq!(self.explicit_on_delete(), Some(action));
        self
    }

    fn assert_relation_update_strategy(self, action: ReferentialAction) -> Self {
        assert_eq!(self.explicit_on_update(), Some(action));
        self
    }
}

impl<'a> ModelAssert<'a> for walkers::ModelWalker<'a> {
    fn assert_field_count(self, count: usize) -> Self {
        assert_eq!(self.scalar_fields().count() + self.relation_fields().count(), count);
        self
    }

    fn assert_ignored(self, ignored: bool) -> Self {
        assert_eq!(self.is_ignored(), ignored);
        self
    }

    #[track_caller]
    fn assert_has_relation_field(self, t: &str) -> walkers::RelationFieldWalker<'a> {
        self.relation_fields()
            .find(|sf| sf.name() == t)
            .expect("Could not find scalar field with name {t}")
    }

    #[track_caller]
    fn assert_has_scalar_field(self, t: &str) -> walkers::ScalarFieldWalker<'a> {
        self.scalar_fields()
            .find(|sf| sf.name() == t)
            .expect("Could not find scalar field with name {t}")
    }
}

impl<'a> ScalarFieldAssert for walkers::ScalarFieldWalker<'a> {
    fn assert_ignored(self, ignored: bool) -> Self {
        assert_eq!(self.is_ignored(), ignored);
        self
    }

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
