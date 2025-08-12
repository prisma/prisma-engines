use alloc::vec::Vec;

use crate::field::ScalarFieldRef;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ScalarProjection {
    /// A single field projection.
    Single(ScalarFieldRef),

    /// A tuple projection, e.g. if (a, b) <in> ((1, 2), (1, 3), ...) is supposed to be queried.
    Compound(Vec<ScalarFieldRef>),
}

impl core::fmt::Debug for ScalarProjection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Single(sf) => f.debug_tuple("SingleProjection").field(&format!("{sf}")).finish(),
            Self::Compound(sfs) => {
                let mut dbg = f.debug_tuple("CompoundProjection");

                for sf in sfs {
                    dbg.field(&format!("{sf}"));
                }

                dbg.finish()
            }
        }
    }
}

impl ScalarProjection {
    pub fn scalar_fields(&self) -> Vec<&ScalarFieldRef> {
        match self {
            ScalarProjection::Single(sf) => vec![sf],
            ScalarProjection::Compound(sfs) => sfs.iter().collect(),
        }
    }

    pub fn as_single(&self) -> Option<&ScalarFieldRef> {
        match self {
            ScalarProjection::Single(sf) => Some(sf),
            ScalarProjection::Compound(_) => None,
        }
    }
}
