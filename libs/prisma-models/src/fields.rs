use crate::*;
use once_cell::sync::OnceCell;
use std::{
    collections::BTreeSet,
    sync::{Arc, Weak},
};

#[derive(Debug)]
pub struct Fields {
    pub all: Vec<Field>,
    id: OnceCell<Option<Vec<ScalarFieldWeak>>>,
    id_field_names: Vec<String>,
    scalar: OnceCell<Vec<ScalarFieldWeak>>,
    relation: OnceCell<Vec<RelationFieldWeak>>,
    model: ModelWeakRef,
    created_at: OnceCell<Option<ScalarFieldRef>>,
    updated_at: OnceCell<Option<ScalarFieldRef>>,
}

impl Fields {
    pub fn new(all: Vec<Field>, model: ModelWeakRef, id_field_names: Vec<String>) -> Fields {
        Fields {
            all,
            id: OnceCell::new(),
            id_field_names,
            scalar: OnceCell::new(),
            relation: OnceCell::new(),
            created_at: OnceCell::new(),
            updated_at: OnceCell::new(),
            model,
        }
    }

    pub fn id(&self) -> Option<Vec<ScalarFieldRef>> {
        self.id
            .get_or_init(|| {
                self.find_singular_id()
                    .map(|x| vec![x])
                    .or_else(|| self.find_multipart_id())
            })
            .clone()
            .map(|fields| fields.into_iter().map(|x| x.upgrade().unwrap()).collect())
    }

    pub fn created_at(&self) -> &Option<ScalarFieldRef> {
        self.created_at.get_or_init(|| {
            self.scalar_weak()
                .iter()
                .map(|sf| sf.upgrade().unwrap())
                .find(|sf| sf.is_created_at())
        })
    }

    pub fn updated_at(&self) -> &Option<ScalarFieldRef> {
        self.updated_at.get_or_init(|| {
            self.scalar_weak()
                .iter()
                .map(|sf| sf.upgrade().unwrap())
                .find(|sf| sf.is_updated_at())
        })
    }

    pub fn scalar(&self) -> Vec<ScalarFieldRef> {
        self.scalar_weak().iter().map(|f| f.upgrade().unwrap()).collect()
    }

    pub fn scalar_non_list(&self) -> Vec<ScalarFieldRef> {
        self.scalar().into_iter().filter(|sf| !sf.is_list).collect()
    }

    pub fn scalar_list(&self) -> Vec<ScalarFieldRef> {
        self.scalar().into_iter().filter(|sf| sf.is_list).collect()
    }

    fn scalar_weak(&self) -> &[ScalarFieldWeak] {
        self.scalar
            .get_or_init(|| self.all.iter().fold(Vec::new(), Self::scalar_filter))
            .as_slice()
    }

    pub fn relation(&self) -> Vec<Arc<RelationField>> {
        self.relation_weak().iter().map(|f| f.upgrade().unwrap()).collect()
    }

    pub fn cascading_relation(&self) -> Vec<Arc<RelationField>> {
        self.relation_weak()
            .iter()
            .map(|f| f.upgrade().unwrap())
            .fold(Vec::new(), |mut acc, rf| {
                match rf.relation_side {
                    RelationSide::A if rf.relation().model_a_on_delete.is_cascade() => acc.push(rf),
                    RelationSide::B if rf.relation().model_b_on_delete.is_cascade() => acc.push(rf),
                    _ => (),
                }

                acc
            })
    }

    fn relation_weak(&self) -> &[Weak<RelationField>] {
        self.relation
            .get_or_init(|| self.all.iter().fold(Vec::new(), Self::relation_filter))
            .as_slice()
    }

    pub fn find_many_from_all(&self, names: &BTreeSet<String>) -> Vec<&Field> {
        self.all.iter().filter(|field| names.contains(field.name())).collect()
    }

    pub fn find_many_from_scalar(&self, names: &BTreeSet<String>) -> Vec<ScalarFieldRef> {
        self.scalar_weak()
            .iter()
            .filter(|field| names.contains(&field.upgrade().unwrap().name))
            .map(|field| field.upgrade().unwrap())
            .collect()
    }

    pub fn find_many_from_relation(&self, names: &BTreeSet<String>) -> Vec<Arc<RelationField>> {
        self.relation_weak()
            .iter()
            .filter(|field| names.contains(&field.upgrade().unwrap().name))
            .map(|field| field.upgrade().unwrap())
            .collect()
    }

    pub fn find_from_all(&self, name: &str) -> DomainResult<&Field> {
        self.all
            .iter()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::FieldNotFound {
                name: name.to_string(),
                model: self.model().name.clone(),
            })
    }

    pub fn find_from_scalar(&self, name: &str) -> DomainResult<ScalarFieldRef> {
        self.scalar_weak()
            .iter()
            .map(|field| field.upgrade().unwrap())
            .find(|field| field.name == name)
            .ok_or_else(|| DomainError::ScalarFieldNotFound {
                name: name.to_string(),
                model: self.model().name.clone(),
            })
    }

    fn model(&self) -> ModelRef {
        self.model.upgrade().unwrap()
    }

    pub fn find_from_relation_fields(&self, name: &str) -> DomainResult<Arc<RelationField>> {
        self.relation_weak()
            .iter()
            .map(|field| field.upgrade().unwrap())
            .find(|field| field.name == name)
            .ok_or_else(|| DomainError::RelationFieldNotFound {
                name: name.to_string(),
                model: self.model().name.clone(),
            })
    }

    pub fn find_from_relation(&self, name: &str, side: RelationSide) -> DomainResult<Arc<RelationField>> {
        self.relation_weak()
            .iter()
            .map(|field| field.upgrade().unwrap())
            .find(|field| field.relation().name == name && field.relation_side == side)
            .ok_or_else(|| DomainError::FieldForRelationNotFound {
                relation: name.to_string(),
                model: self.model().name.clone(),
            })
    }

    fn scalar_filter(mut acc: Vec<ScalarFieldWeak>, field: &Field) -> Vec<ScalarFieldWeak> {
        if let Field::Scalar(scalar_field) = field {
            acc.push(Arc::downgrade(scalar_field));
        };

        acc
    }

    fn relation_filter<'a>(mut acc: Vec<Weak<RelationField>>, field: &'a Field) -> Vec<Weak<RelationField>> {
        if let Field::Relation(relation_field) = field {
            acc.push(Arc::downgrade(relation_field));
        };

        acc
    }

    fn find_singular_id(&self) -> Option<ScalarFieldWeak> {
        self.scalar_weak().into_iter().find_map(|wsf| {
            let sf = wsf.upgrade().unwrap();

            if sf.is_id() {
                Some(Weak::clone(wsf))
            } else {
                None
            }
        })
    }

    fn find_multipart_id(&self) -> Option<Vec<ScalarFieldWeak>> {
        if self.id_field_names.len() > 0 {
            let scalars = self.scalar();
            let fields = self
                .id_field_names
                .iter()
                .map(|f| {
                    let id_field = scalars
                        .iter()
                        .find(|sf| &sf.name == f)
                        .expect(&format!("Expected ID field {} to be present on the model", f));
                    Arc::downgrade(id_field)
                })
                .collect();

            Some(fields)
        // Some(self.scalar().filter_map(|sf| x).collect())
        } else {
            None
        }
    }
}
