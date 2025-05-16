use crate::{psl::parser_database::walkers::Walker, InternalDataModelRef, TypeIdentifier};
use std::hash::{Hash, Hasher};

// Invariant: InternalDataModel must not contain any Zipper, this would be a reference counting
// cycle (memory leak).
#[derive(Clone)]
pub struct Zipper<I> {
    pub id: I,
    pub dm: InternalDataModelRef,
}

impl<I: PartialEq> PartialEq for Zipper<I> {
    #[allow(unconditional_recursion)]
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl<I: Eq> Eq for Zipper<I> {}

impl<I: PartialOrd> PartialOrd for Zipper<I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl<I: Ord> Ord for Zipper<I> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<I: Copy> Zipper<I> {
    pub fn walker(&self) -> Walker<'_, I> {
        self.dm.schema.db.walk(self.id)
    }
}

impl<I: Hash> Hash for Zipper<I> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl std::fmt::Debug for Zipper<TypeIdentifier> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TypeIdentifier")
            .field(&format!("{:?}", self.id))
            .finish()
    }
}
