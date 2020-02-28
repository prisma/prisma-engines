use super::ModelIdentifier;
use crate::{DataSourceFieldRef, DomainError, PrismaValue};
use std::{collections::HashMap, convert::TryFrom};

/// Collection of field to value pairs corresponding to a ModelIdentifier the record belongs to.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordIdentifier {
    pub pairs: Vec<(DataSourceFieldRef, PrismaValue)>,
}

impl RecordIdentifier {
    pub fn new(pairs: Vec<(DataSourceFieldRef, PrismaValue)>) -> Self {
        Self { pairs }
    }

    pub fn add(&mut self, pair: (DataSourceFieldRef, PrismaValue)) {
        self.pairs.push(pair);
    }

    pub fn fields(&self) -> impl Iterator<Item = DataSourceFieldRef> + '_ {
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

        return false;
    }

    /// Consumes this identifier and splits it into a set of `RecordIdentifier`s based on the passed
    /// `ModelIdentifier`s. Assumes that The transformation can be done.
    pub fn split_into(self, identifiers: &[ModelIdentifier]) -> Vec<RecordIdentifier> {
        let mapped: HashMap<String, (DataSourceFieldRef, PrismaValue)> = self
            .into_iter()
            .map(|(dsf, val)| (dsf.name.clone(), (dsf, val)))
            .collect();

        identifiers
            .into_iter()
            .map(|ident| {
                ident
                    .data_source_fields()
                    .map(|dsf| {
                        let entry = mapped
                            .get(&dsf.name)
                            .expect("Error splitting RecordIdentifier: ModelIdentifier doesn't match.")
                            .clone();

                        entry
                    })
                    .collect::<Vec<_>>()
                    .into()
            })
            .collect()
    }

    // [DTODO] Remove
    pub fn single_value(&self) -> PrismaValue {
        assert_eq!(
            self.pairs.len(),
            1,
            "This function must only be called on singular record identifiers"
        );
        self.pairs.iter().next().unwrap().1.clone()
    }
}

impl TryFrom<RecordIdentifier> for PrismaValue {
    type Error = DomainError;

    fn try_from(id: RecordIdentifier) -> crate::Result<Self> {
        match id.pairs.into_iter().next() {
            Some(value) => Ok(value.1),
            None => Err(DomainError::ConversionFailure(
                "RecordIdentifier".into(),
                "PrismaValue".into(),
            )),
        }
    }
}

impl IntoIterator for RecordIdentifier {
    type Item = (DataSourceFieldRef, PrismaValue);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.pairs.into_iter()
    }
}

impl From<(DataSourceFieldRef, PrismaValue)> for RecordIdentifier {
    fn from(tup: (DataSourceFieldRef, PrismaValue)) -> Self {
        Self::new(vec![tup])
    }
}

impl From<Vec<(DataSourceFieldRef, PrismaValue)>> for RecordIdentifier {
    fn from(tup: Vec<(DataSourceFieldRef, PrismaValue)>) -> Self {
        Self::new(tup)
    }
}
