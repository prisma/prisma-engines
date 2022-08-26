use crate::{parent_container::ParentContainer, CompositeTypeRef, Index};
use datamodel::dml::FieldArity;
use once_cell::sync::OnceCell;
use std::{
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type CompositeFieldRef = Arc<CompositeField>;
pub type CompositeFieldWeak = Weak<CompositeField>;

#[derive(Clone)]
pub struct CompositeField {
    pub name: String,
    pub typ: CompositeTypeRef,
    pub unique_index: OnceCell<Option<Index>>,
    pub(crate) db_name: Option<String>,
    pub(crate) arity: FieldArity,
    pub(crate) container: ParentContainer,
}

impl CompositeField {
    pub fn is_list(&self) -> bool {
        matches!(self.arity, FieldArity::List)
    }

    pub fn is_required(&self) -> bool {
        matches!(self.arity, FieldArity::Required)
    }

    pub fn is_optional(&self) -> bool {
        matches!(self.arity, FieldArity::Optional)
    }

    pub fn db_name(&self) -> &str {
        self.db_name.as_deref().unwrap_or(self.name.as_str())
    }

    pub fn container(&self) -> &ParentContainer {
        &self.container
    }

    pub fn unique_index(&self) -> &Option<Index> {
        self.unique_index.get_or_init(|| None)
    }

    pub fn is_unique(&self) -> bool {
        self.unique_index().is_some()
    }

    pub fn arity(&self) -> FieldArity {
        self.arity
    }
}

impl Debug for CompositeField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeField")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .field("container", &self.container)
            .field("composite_type", &self.typ.name)
            .finish()
    }
}

impl Display for CompositeField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.container().name(), self.name)
    }
}

impl Hash for CompositeField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Names are unique in the data model.
        self.name.hash(state);
    }
}

impl Eq for CompositeField {}

impl PartialEq for CompositeField {
    fn eq(&self, other: &CompositeField) -> bool {
        self.name == other.name
    }
}
