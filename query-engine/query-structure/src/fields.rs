use crate::*;
use psl::parser_database::ScalarFieldType;

#[derive(Debug, Clone)]
pub struct Fields<'a> {
    model: &'a Model,
}

impl<'a> Fields<'a> {
    pub(crate) fn new(model: &'a Model) -> Fields<'a> {
        Fields { model }
    }

    pub fn id_fields(&self) -> Option<impl Iterator<Item = ScalarFieldRef> + Clone + 'a> {
        let dm = &self.model.dm;
        self.model.walker().primary_key().map(move |pk| {
            pk.fields()
                .map(move |field| dm.clone().zip(ScalarFieldId::InModel(field.id)))
        })
    }

    pub fn compound_id(&self) -> Option<impl Iterator<Item = ScalarFieldRef> + Clone + '_> {
        self.model
            .walker()
            .primary_key()
            .filter(|pk| pk.fields().len() > 1)
            .map(|pk| {
                pk.fields()
                    .map(|field| self.model.dm.clone().zip(ScalarFieldId::InModel(field.id)))
            })
    }

    pub fn updated_at(&self) -> impl Iterator<Item = ScalarFieldRef> + 'a {
        self.scalar().filter(|sf| sf.is_updated_at())
    }

    pub fn scalar(&self) -> impl Iterator<Item = ScalarFieldRef> + Clone + 'a {
        self.model
            .dm
            .walk(self.model.id)
            .scalar_fields()
            .filter(|sf| {
                !sf.is_ignored()
                    && !matches!(
                        sf.scalar_field_type(),
                        ScalarFieldType::CompositeType(_) | ScalarFieldType::Unsupported(_)
                    )
            })
            .map(|rf| self.model.dm.clone().zip(ScalarFieldId::InModel(rf.id)))
    }

    pub fn relation(&self) -> impl Iterator<Item = RelationFieldRef> + 'a {
        self.model
            .dm
            .walk(self.model.id)
            .relation_fields()
            .filter(|rf| !rf.relation().is_ignored())
            .map(|rf| self.model.dm.clone().zip(rf.id))
    }

    pub fn composite(&self) -> Vec<CompositeFieldRef> {
        self.model
            .dm
            .walk(self.model.id)
            .scalar_fields()
            .filter(|sf| sf.scalar_field_type().as_composite_type().is_some() && !sf.is_ignored())
            .map(|sf| self.model.dm.clone().zip(CompositeFieldId::InModel(sf.id)))
            .collect()
    }

    pub fn non_relational(&self) -> Vec<Field> {
        self.scalar()
            .map(Field::from)
            .chain(self.composite().into_iter().map(Field::from))
            .collect()
    }

    pub fn find_from_all(&self, prisma_name: &str) -> crate::Result<Field> {
        let model_walker = self.model.walker();
        let mut scalar_fields = model_walker.scalar_fields();
        let mut relation_fields = model_walker.relation_fields();
        scalar_fields
            .find(|f| f.name() == prisma_name)
            .map(|w| Field::from((self.model.dm.clone(), w)))
            .or_else(|| {
                relation_fields
                    .find(|f| f.name() == prisma_name)
                    .map(|w| Field::from((self.model.dm.clone(), w)))
            })
            .ok_or_else(|| DomainError::FieldNotFound {
                name: prisma_name.to_string(),
                container_name: self.model().name().to_owned(),
                container_type: "model",
            })
    }

    /// Non-virtual: Fields actually existing on the database level, this (currently) excludes relations, which are
    /// purely virtual on a model.
    pub fn find_from_non_virtual_by_db_name(&self, db_name: &str) -> crate::Result<Field> {
        self.filter_all(|f| f.db_name() == db_name)
            .next()
            .ok_or_else(|| DomainError::FieldNotFound {
                name: db_name.to_string(),
                container_name: self.model().name().to_owned(),
                container_type: "model",
            })
    }

    pub fn find_from_scalar(&self, name: &str) -> crate::Result<ScalarFieldRef> {
        self.scalar()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::ScalarFieldNotFound {
                name: name.to_string(),
                container_name: self.model().name().to_owned(),
                container_type: "model",
            })
    }

    fn model(&self) -> &Model {
        self.model
    }

    pub fn find_from_relation_fields(&self, name: &str) -> Result<RelationFieldRef> {
        self.relation()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::RelationFieldNotFound {
                name: name.to_string(),
                model: self.model().name().to_owned(),
            })
    }

    pub fn all(&self) -> impl Iterator<Item = Field> + 'a {
        let dm = &self.model.dm;
        let model_walker = dm.walk(self.model.id);
        model_walker
            .scalar_fields()
            .filter(|f| !f.is_ignored() && !f.is_unsupported())
            .map(|w| Field::from((dm.clone(), w)))
            .chain(
                model_walker
                    .relation_fields()
                    .filter(|rf| !rf.relation().is_ignored())
                    .map(|w| Field::from((dm.clone(), w))),
            )
    }

    pub fn filter_all<P>(&self, predicate: P) -> impl Iterator<Item = Field> + 'a
    where
        P: Fn(&&Field) -> bool + 'a,
    {
        self.all().filter(move |f| predicate(&f))
    }
}
