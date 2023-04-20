use crate::*;
use psl::parser_database::ScalarFieldType;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct Fields {
    model: Model,
}

impl Fields {
    pub(crate) fn new(model: Model) -> Fields {
        Fields { model }
    }

    pub fn id_fields(&self) -> Option<impl Iterator<Item = ScalarFieldRef> + Clone + '_> {
        self.model.walker().primary_key().map(|pk| {
            pk.fields()
                .map(move |field| self.model.dm.clone().zip(ScalarFieldId::InModel(field.id)))
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

    pub fn updated_at(&self) -> impl Iterator<Item = ScalarFieldRef> {
        self.scalar().into_iter().filter(|sf| sf.is_updated_at())
    }

    pub fn scalar(&self) -> Vec<ScalarFieldRef> {
        self.model
            .dm
            .walk(self.model.id)
            .scalar_fields()
            .filter(|sf| {
                !matches!(
                    sf.scalar_field_type(),
                    ScalarFieldType::CompositeType(_) | ScalarFieldType::Unsupported(_)
                )
            })
            .map(|rf| self.model.dm.clone().zip(ScalarFieldId::InModel(rf.id)))
            .collect()
    }

    pub fn relation(&self) -> Vec<RelationFieldRef> {
        self.model
            .dm
            .walk(self.model.id)
            .relation_fields()
            .filter(|rf| !rf.relation().is_ignored())
            .map(|rf| self.model.dm.clone().zip(rf.id))
            .collect()
    }

    pub fn composite(&self) -> Vec<CompositeFieldRef> {
        self.model
            .dm
            .walk(self.model.id)
            .scalar_fields()
            .filter(|sf| sf.scalar_field_type().as_composite_type().is_some())
            .map(|sf| self.model.dm.clone().zip(CompositeFieldId::InModel(sf.id)))
            .collect()
    }

    pub fn non_relational(&self) -> Vec<Field> {
        self.scalar()
            .into_iter()
            .map(Field::from)
            .chain(self.composite().into_iter().map(Field::from))
            .collect()
    }

    pub fn find_many_from_scalar(&self, names: &BTreeSet<String>) -> Vec<ScalarFieldRef> {
        self.scalar()
            .into_iter()
            .filter(|field| names.contains(field.name()))
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
            .into_iter()
            .next()
            .ok_or_else(|| DomainError::FieldNotFound {
                name: db_name.to_string(),
                container_name: self.model().name().to_owned(),
                container_type: "model",
            })
    }

    pub fn find_from_scalar(&self, name: &str) -> crate::Result<ScalarFieldRef> {
        self.scalar()
            .into_iter()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::ScalarFieldNotFound {
                name: name.to_string(),
                container_name: self.model().name().to_owned(),
                container_type: "model",
            })
    }

    fn model(&self) -> &ModelRef {
        &self.model
    }

    pub fn find_from_relation_fields(&self, name: &str) -> Result<RelationFieldRef> {
        self.relation()
            .into_iter()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::RelationFieldNotFound {
                name: name.to_string(),
                model: self.model().name().to_owned(),
            })
    }

    pub fn filter_all<P>(&self, predicate: P) -> Vec<Field>
    where
        P: Fn(&&Field) -> bool,
    {
        let model_walker = self.model.walker();
        model_walker
            .scalar_fields()
            .filter(|f| !f.is_ignored() && !f.is_unsupported())
            .map(|w| Field::from((self.model.dm.clone(), w)))
            .chain(
                model_walker
                    .relation_fields()
                    .filter(|rf| !rf.relation().is_ignored())
                    .map(|w| Field::from((self.model.dm.clone(), w))),
            )
            .filter(|f| predicate(&f))
            .collect()
    }
}
