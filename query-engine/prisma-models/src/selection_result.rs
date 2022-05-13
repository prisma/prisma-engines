use crate::{DomainError, FieldSelection, PrismaValue, ScalarFieldRef, SelectedField};
use itertools::Itertools;
use std::convert::TryFrom;

/// Represents a set of results.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct SelectionResult {
    pub pairs: Vec<(SelectedField, PrismaValue)>,
}

impl std::fmt::Debug for SelectionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(
                &self
                    .pairs
                    .iter()
                    .map(|pair| (format!("{}", pair.0), pair.1.clone()))
                    .collect_vec(),
            )
            .finish()
    }
}

impl SelectionResult {
    pub fn new<T>(pairs: Vec<(T, PrismaValue)>) -> Self
    where
        T: Into<SelectedField>,
    {
        Self {
            pairs: pairs.into_iter().map(|(rt, value)| (rt.into(), value)).collect(),
        }
    }

    pub fn add<T>(&mut self, pair: (T, PrismaValue))
    where
        T: Into<SelectedField>,
    {
        self.pairs.push((pair.0.into(), pair.1));
    }

    pub fn get(&self, selection: &SelectedField) -> Option<&PrismaValue> {
        self.pairs.iter().find_map(|(result_selection, value)| {
            if selection == result_selection {
                Some(value)
            } else {
                None
            }
        })
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

    /// Consumes this `SelectionResult` and splits it into a set of `SelectionResult`s based on the passed
    /// `FieldSelection`s. Assumes that the transformation can be done.
    pub fn split_into(self, field_selections: &[FieldSelection]) -> Vec<SelectionResult> {
        field_selections
            .iter()
            .map(|field_selection| {
                let pairs: Vec<_> = field_selection
                    .selections()
                    .map(|selected_field| {
                        self.get(selected_field)
                            .map(|value| (selected_field.clone(), value.clone()))
                            .expect("Error splitting `ReturnValues`: `FieldSelection` doesn't match.")
                    })
                    .collect();

                SelectionResult::new(pairs)
            })
            .collect()
    }

    /// Checks if `self` only contains scalar field selections and if so, returns them all in a list.
    /// If any other selection is contained, returns `None`.
    pub fn as_scalar_fields(&self) -> Option<Vec<ScalarFieldRef>> {
        let scalar_fields: Vec<_> = self
            .pairs
            .iter()
            .filter_map(|(selection, _)| match selection {
                SelectedField::Scalar(sf) => Some(sf.clone()),
                SelectedField::Composite(_) => None,
            })
            .collect();

        if scalar_fields.len() == self.pairs.len() {
            Some(scalar_fields)
        } else {
            None
        }
    }

    /// Coerces contained values to best fit their type.
    /// - Scalar fields coerce values based on the TypeIdentifier.
    /// - Composite fields must be objects and contained fields must also follow the type coherence.
    pub fn coerce_values(self) -> crate::Result<Self> {
        let pairs = self
            .pairs
            .into_iter()
            .map(|(selection, value)| {
                let value = selection.coerce_value(value)?;
                Ok((selection, value))
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(Self { pairs })
    }
}

impl TryFrom<SelectionResult> for PrismaValue {
    type Error = DomainError;

    fn try_from(return_values: SelectionResult) -> crate::Result<Self> {
        match return_values.pairs.into_iter().next() {
            Some(value) => Ok(value.1),
            None => Err(DomainError::ConversionFailure(
                "ReturnValues".into(),
                "PrismaValue".into(),
            )),
        }
    }
}

impl IntoIterator for SelectionResult {
    type Item = (SelectedField, PrismaValue);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.pairs.into_iter()
    }
}

impl<T> From<(T, PrismaValue)> for SelectionResult
where
    T: Into<SelectedField>,
{
    fn from((x, value): (T, PrismaValue)) -> Self {
        Self::new(vec![(x.into(), value)])
    }
}

impl<T> From<Vec<(T, PrismaValue)>> for SelectionResult
where
    T: Into<SelectedField>,
{
    fn from(tuples: Vec<(T, PrismaValue)>) -> Self {
        Self::new(tuples.into_iter().map(|(x, value)| (x.into(), value)).collect())
    }
}

impl From<&FieldSelection> for SelectionResult {
    fn from(fs: &FieldSelection) -> Self {
        Self {
            pairs: fs
                .selections()
                .map(|selection| (selection.clone(), PrismaValue::Null))
                .collect(),
        }
    }
}
