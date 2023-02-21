use crate::pk::PrimaryKey;
use crate::*;
use once_cell::sync::OnceCell;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Debug, Clone)]
pub struct Fields {
    all: Vec<Field>,
    primary_key: Option<PrimaryKey>,
    scalar: OnceCell<Vec<ScalarFieldWeak>>,
    model: ModelWeakRef,
    updated_at: OnceCell<Vec<ScalarFieldRef>>,
}

impl Fields {
    pub(crate) fn new(all: Vec<Field>, model: ModelWeakRef, primary_key: Option<PrimaryKey>) -> Fields {
        Fields {
            all,
            primary_key,
            scalar: OnceCell::new(),
            updated_at: OnceCell::new(),
            model,
        }
    }

    pub fn id(&self) -> Option<&PrimaryKey> {
        self.primary_key.as_ref()
    }

    pub fn compound_id(&self) -> Option<&PrimaryKey> {
        if self
            .primary_key
            .as_ref()
            .map(|pk| pk.fields().len() > 1)
            .unwrap_or(false)
        {
            self.primary_key.as_ref()
        } else {
            None
        }
    }

    pub fn updated_at(&self) -> &Vec<ScalarFieldRef> {
        self.updated_at.get_or_init(|| {
            self.scalar_weak()
                .iter()
                .map(|sf| sf.upgrade().unwrap())
                .filter(|sf| sf.is_updated_at)
                .collect()
        })
    }

    pub fn scalar(&self) -> Vec<ScalarFieldRef> {
        self.scalar_weak().iter().map(|f| f.upgrade().unwrap()).collect()
    }

    fn scalar_weak(&self) -> &[ScalarFieldWeak] {
        self.scalar
            .get_or_init(|| self.all.iter().fold(Vec::new(), Self::scalar_filter))
            .as_slice()
    }

    pub fn relation(&self) -> Vec<RelationFieldRef> {
        let model = self.model();
        let internal_data_model = model.internal_data_model();
        internal_data_model
            .walk(model.id)
            .relation_fields()
            .map(|rf| internal_data_model.clone().zip(rf.id))
            .collect()
    }

    fn composite(&self) -> Vec<CompositeFieldRef> {
        let model = self.model();
        let internal_data_model = model.internal_data_model();
        internal_data_model
            .walk(model.id)
            .scalar_fields()
            .filter(|sf| sf.scalar_field_type().as_composite_type().is_some())
            .map(|sf| internal_data_model.clone().zip(CompositeFieldId::InModel(sf.id)))
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
        self.scalar_weak()
            .iter()
            .filter(|field| names.contains(&field.upgrade().unwrap().name))
            .map(|field| field.upgrade().unwrap())
            .collect()
    }

    pub fn find_from_all(&self, prisma_name: &str) -> crate::Result<Field> {
        self.all
            .iter()
            .find(|field| field.name() == prisma_name)
            .map(Clone::clone)
            .or_else(|| {
                let model = self.model();
                let internal_data_model = model.internal_data_model();
                let id = internal_data_model
                    .walk(model.id)
                    .scalar_fields()
                    .find(|sf| sf.name() == prisma_name)
                    .map(|sf| sf.id);
                id.map(|id| Field::from(internal_data_model.clone().zip(CompositeFieldId::InModel(id))))
            })
            .or_else(|| {
                let model = self.model();
                let internal_data_model = model.internal_data_model();
                let id = internal_data_model
                    .walk(model.id)
                    .relation_fields()
                    .find(|rf| rf.name() == prisma_name)
                    .map(|rf| rf.id);
                id.map(|id| Field::from(internal_data_model.clone().zip(id)))
            })
            .ok_or_else(|| DomainError::FieldNotFound {
                name: prisma_name.to_string(),
                container_name: self.model().name.clone(),
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
                container_name: self.model().name.clone(),
                container_type: "model",
            })
    }

    pub fn find_from_scalar(&self, name: &str) -> crate::Result<ScalarFieldRef> {
        self.scalar()
            .into_iter()
            .find(|field| field.name == name)
            .ok_or_else(|| DomainError::ScalarFieldNotFound {
                name: name.to_string(),
                container_name: self.model().name.clone(),
                container_type: "model",
            })
    }

    fn model(&self) -> ModelRef {
        self.model.upgrade().unwrap()
    }

    pub fn find_from_relation_fields(&self, name: &str) -> Result<RelationFieldRef> {
        self.relation()
            .into_iter()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::RelationFieldNotFound {
                name: name.to_string(),
                model: self.model().name.clone(),
            })
    }

    fn scalar_filter(mut acc: Vec<ScalarFieldWeak>, field: &Field) -> Vec<ScalarFieldWeak> {
        if let Field::Scalar(scalar_field) = field {
            acc.push(Arc::downgrade(scalar_field));
        };

        acc
    }

    pub fn filter_all<P>(&self, predicate: P) -> Vec<Field>
    where
        P: Fn(&&Field) -> bool,
    {
        let model = self.model();
        let internal_data_model = model.internal_data_model();
        let rf = internal_data_model
            .walk(model.id)
            .relation_fields()
            .map(|rf| Field::from(internal_data_model.clone().zip(rf.id)))
            .filter(|f| predicate(&f));
        let composite_type_fields = internal_data_model
            .walk(model.id)
            .scalar_fields()
            .filter(|sf| sf.scalar_field_type().as_composite_type().is_some())
            .map(|sf| Field::from(internal_data_model.clone().zip(CompositeFieldId::InModel(sf.id))))
            .filter(|f| predicate(&f));
        self.all
            .iter()
            .filter(&predicate)
            .map(Clone::clone)
            .chain(rf)
            .chain(composite_type_fields)
            .collect()
    }
}
