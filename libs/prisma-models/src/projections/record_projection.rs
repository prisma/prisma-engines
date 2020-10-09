use super::ModelProjection;
use crate::{DomainError, PrismaValue, ScalarFieldRef};
use std::{collections::HashMap, convert::TryFrom};

/// Represents a (sub)set of fields to value pairs from a single record.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordProjection {
    pub pairs: Vec<(ScalarFieldRef, PrismaValue)>,
}

impl RecordProjection {
    pub fn new(pairs: Vec<(ScalarFieldRef, PrismaValue)>) -> Self {
        Self { pairs }
    }

    pub fn add(&mut self, pair: (ScalarFieldRef, PrismaValue)) {
        self.pairs.push(pair);
    }

    pub fn fields(&self) -> impl Iterator<Item = ScalarFieldRef> + '_ {
        self.pairs.iter().map(|p| p.0.clone())
    }

    pub fn values(&self) -> impl Iterator<Item = PrismaValue> + '_ {
        self.pairs.iter().map(|p| p.1.clone())
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn misses_autogen_value(&self) -> bool {
        self.pairs.iter().any(|p| p.1.is_null())
    }

    pub fn add_autogen_value<V>(&mut self, value: V) -> bool
    where
        V: Into<PrismaValue>,
    {
        for pair in self.pairs.iter_mut() {
            if pair.1.is_null() {
                pair.1 = value.into();
                return true;
            }
        }

        false
    }

    /// Consumes this projection and splits it into a set of `RecordProjection`s based on the passed
    /// `ModelProjection`s. Assumes that the transformation can be done.
    pub fn split_into(self, projections: &[ModelProjection]) -> Vec<RecordProjection> {
        let mapped: HashMap<String, (ScalarFieldRef, PrismaValue)> =
            self.into_iter().map(|(sf, val)| (sf.name.clone(), (sf, val))).collect();

        projections
            .iter()
            .map(|p| {
                p.scalar_fields()
                    .map(|sf| {
                        let entry = mapped
                            .get(&sf.name)
                            .expect("Error splitting RecordProjection: ModelProjection doesn't match.")
                            .clone();

                        entry
                    })
                    .collect::<Vec<_>>()
                    .into()
            })
            .collect()
    }
}

impl TryFrom<RecordProjection> for PrismaValue {
    type Error = DomainError;

    fn try_from(projection: RecordProjection) -> crate::Result<Self> {
        match projection.pairs.into_iter().next() {
            Some(value) => Ok(value.1),
            None => Err(DomainError::ConversionFailure(
                "RecordProjection".into(),
                "PrismaValue".into(),
            )),
        }
    }
}

impl IntoIterator for RecordProjection {
    type Item = (ScalarFieldRef, PrismaValue);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.pairs.into_iter()
    }
}

impl From<(ScalarFieldRef, PrismaValue)> for RecordProjection {
    fn from(tup: (ScalarFieldRef, PrismaValue)) -> Self {
        Self::new(vec![tup])
    }
}

impl From<Vec<(ScalarFieldRef, PrismaValue)>> for RecordProjection {
    fn from(tup: Vec<(ScalarFieldRef, PrismaValue)>) -> Self {
        Self::new(tup)
    }
}
