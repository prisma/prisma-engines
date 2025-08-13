use alloc::{borrow::ToOwned, string::String, vec::Vec};
use core::{
    fmt::Debug,
    hash::{Hash, Hasher},
};

use crate::{CompositeType, Field, InternalDataModelRef, Model};

#[derive(Clone)]
pub enum ParentContainer {
    Model(Model),
    CompositeType(CompositeType),
}

impl ParentContainer {
    pub fn internal_data_model(&self) -> InternalDataModelRef {
        // Unwraps are safe - the models and composites must exist after DML translation.
        match self {
            ParentContainer::Model(model) => model.dm.clone(),
            ParentContainer::CompositeType(composite) => composite.dm.clone(),
        }
    }

    pub fn as_model(&self) -> Option<Model> {
        match self {
            ParentContainer::Model(m) => Some(m.clone()),
            ParentContainer::CompositeType(_) => None,
        }
    }

    pub fn as_composite(&self) -> Option<CompositeType> {
        match self {
            ParentContainer::Model(_) => None,
            ParentContainer::CompositeType(ct) => Some(ct.clone()),
        }
    }

    pub fn name(&self) -> String {
        match self {
            ParentContainer::Model(model) => model.name().to_owned(),
            ParentContainer::CompositeType(composite) => composite.walker().name().to_owned(),
        }
    }

    pub fn fields(&self) -> Vec<Field> {
        match self {
            ParentContainer::Model(model) => model.fields().all().collect(),
            ParentContainer::CompositeType(composite) => composite.fields().collect(),
        }
    }

    pub fn find_field(&self, prisma_name: &str) -> Option<Field> {
        match self {
            ParentContainer::Model(model) => model.fields().find_from_all(prisma_name).ok(),
            ParentContainer::CompositeType(ct) => ct.fields().find(|field| field.name() == prisma_name),
        }
    }

    pub fn is_composite(&self) -> bool {
        matches!(self, Self::CompositeType(..))
    }

    pub fn is_model(&self) -> bool {
        matches!(self, Self::Model(..))
    }
}

impl From<&Model> for ParentContainer {
    fn from(model: &Model) -> Self {
        Self::Model(model.clone())
    }
}

impl From<Model> for ParentContainer {
    fn from(model: Model) -> Self {
        Self::Model(model)
    }
}

impl From<CompositeType> for ParentContainer {
    fn from(composite: CompositeType) -> Self {
        Self::CompositeType(composite)
    }
}

impl Debug for ParentContainer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParentContainer::Model(m) => f
                .debug_struct("ParentContainer")
                .field("enum_variant", &"Model")
                .field("name", &m.name())
                .finish(),

            ParentContainer::CompositeType(ct) => f
                .debug_struct("ParentContainer")
                .field("enum_variant", &"CompositeType")
                .field("name", &ct.walker().name())
                .finish(),
        }
    }
}

impl Hash for ParentContainer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ParentContainer::Model(model) => model.hash(state),
            ParentContainer::CompositeType(composite) => composite.hash(state),
        }
    }
}

impl Eq for ParentContainer {}

impl PartialEq for ParentContainer {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ParentContainer::Model(id_a), ParentContainer::Model(id_b)) => id_a == id_b,
            (ParentContainer::CompositeType(id_a), ParentContainer::CompositeType(id_b)) => id_a == id_b,
            _ => false,
        }
    }
}
