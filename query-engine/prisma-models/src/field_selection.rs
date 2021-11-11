use std::fmt::Display;

use itertools::Itertools;

use crate::{CompositeFieldRef, DomainError, Field, RelationField, ScalarFieldRef, SelectionResult};

/// A selection of fields from a model.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSelection {
    selections: Vec<SelectedField>,
}

/// A selected field. Can be contained on a model or composite type.
// Todo: Think about virtual selections like aggregations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SelectedField {
    Scalar(ScalarFieldRef),
    Composite(CompositeSelection),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompositeSelection {
    pub field: CompositeFieldRef,
    pub selections: Vec<SelectedField>,
}

impl CompositeSelection {
    pub fn is_superset(&self, other: &Self) -> bool {
        self.field.typ == other.field.typ
            && other.selections.iter().all(|selection| match selection {
                SelectedField::Scalar(sf) => self.contains(&sf.name),
                SelectedField::Composite(other_cs) => self
                    .get(&other_cs.field.name)
                    .and_then(|selection| selection.as_composite())
                    .map(|cs| cs.is_superset(other_cs))
                    .unwrap_or(false),
            })
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn get(&self, name: &str) -> Option<&SelectedField> {
        self.selections.iter().find(|selection| selection.prisma_name() == name)
    }
}

impl FieldSelection {
    /// Returns `true` if self contains (at least) all fields specified in `other`. `false` otherwise.
    /// Recurses into composite selections and ensures that composite selections are supersets as well.
    pub fn is_superset(&self, other: &Self) -> bool {
        other.selections.iter().all(|selection| match selection {
            SelectedField::Scalar(sf) => self.contains(&sf.name),
            SelectedField::Composite(other_cs) => self
                .get(&other_cs.field.name)
                .and_then(|selection| selection.as_composite())
                .map(|cs| cs.is_superset(other_cs))
                .unwrap_or(false),
        })
    }

    pub fn selections(&self) -> impl Iterator<Item = &SelectedField> + '_ {
        self.selections.iter()
    }

    /// Returns all Prisma (e.g. schema model field) names of contained fields.
    /// Does _not_ recurse into composite selections and only iterates top level fields.
    pub fn prisma_names(&self) -> impl Iterator<Item = String> + '_ {
        self.selections.iter().map(|f| f.prisma_name().to_owned())
    }

    /// Returns all database (e.g. column or document field) names of contained fields.
    /// Does _not_ recurse into composite selections and only iterates level fields.
    pub fn db_names(&self) -> impl Iterator<Item = String> + '_ {
        self.selections.iter().map(|f| f.db_name().to_owned())
    }

    /// Checked if a field of prisma name `name` is present in this `FieldSelection`.
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn get(&self, name: &str) -> Option<&SelectedField> {
        self.selections.iter().find(|selection| selection.prisma_name() == name)
    }

    pub fn as_fields(&self) -> Vec<Field> {
        self.selections()
            .map(|selection| match selection {
                SelectedField::Scalar(sf) => sf.clone().into(),
                SelectedField::Composite(cf) => cf.field.clone().into(),
            })
            .collect()
    }

    /// Checks if `self` only contains scalar field selections and if so, returns them all in a list.
    /// If any other selection is contained, returns `None`.
    pub fn as_scalar_fields(&self) -> Option<Vec<ScalarFieldRef>> {
        let scalar_fields = self
            .selections()
            .filter_map(|selection| match selection {
                SelectedField::Scalar(sf) => Some(sf.clone()),
                SelectedField::Composite(_) => None,
            })
            .collect_vec();

        if scalar_fields.len() == self.selections.len() {
            Some(scalar_fields)
        } else {
            None
        }
    }

    /// Inserts this selections fields into the given field values.
    /// Assumes caller knows that the exchange can be done, but still errors if lengths mismatch.
    /// Additionally performs a type coercion based on the source and destination field types.
    /// Resistance is futile.
    pub fn assimilate(&self, values: SelectionResult) -> crate::Result<SelectionResult> {
        if self.selections.len() != values.len() {
            Err(DomainError::ConversionFailure(
                "field values".to_owned(),
                "assimilated field values".to_owned(),
            ))
        } else {
            // let fields = self.as_fields();

            // Ok(values
            //     .pairs
            //     .into_iter()
            //     .zip(fields)
            //     .map(|((og_field, value), other_field)| {
            //         match og_field {

            //         }
            //         if og_field.type_identifier != other_field.type_identifier {
            //             let value = value.coerce(&other_field.type_identifier)?;
            //             Ok((other_field, value))
            //         } else {
            //             Ok((other_field, value))
            //         }
            //     })
            //     .collect::<crate::Result<Vec<_>>>()?
            //     .into())

            todo!()
        }
    }
}

impl SelectedField {
    pub fn prisma_name(&self) -> &str {
        match self {
            SelectedField::Scalar(sf) => &sf.name,
            SelectedField::Composite(cf) => &cf.field.name,
        }
    }

    pub fn db_name(&self) -> &str {
        match self {
            SelectedField::Scalar(sf) => sf.db_name(),
            SelectedField::Composite(cs) => cs.field.db_name(),
        }
    }

    pub fn as_composite(&self) -> Option<&CompositeSelection> {
        match self {
            SelectedField::Composite(ref cs) => Some(cs),
            _ => None,
        }
    }
}

impl From<Vec<Field>> for FieldSelection {
    fn from(fields: Vec<Field>) -> Self {
        Self {
            selections: fields
                .into_iter()
                .flat_map(|field| match field {
                    Field::Relation(rf) => rf.scalar_fields().into_iter().map(Into::into).collect(),
                    Field::Scalar(sf) => vec![sf.into()],
                    Field::Composite(_cf) => todo!(),
                })
                .collect(),
        }
    }
}

impl From<ScalarFieldRef> for FieldSelection {
    fn from(field: ScalarFieldRef) -> Self {
        Self {
            selections: vec![field.into()],
        }
    }
}

impl From<&RelationField> for FieldSelection {
    fn from(rf: &RelationField) -> Self {
        Self {
            selections: rf.scalar_fields().into_iter().map(|sf| sf.into()).collect(),
        }
    }
}

impl From<ScalarFieldRef> for SelectedField {
    fn from(f: ScalarFieldRef) -> Self {
        Self::Scalar(f)
    }
}

impl Display for FieldSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FieldSelection {{ fields: [{}] }}",
            self.selections
                .iter()
                .map(|selection| format!("{}", selection))
                .join(", ")
        )
    }
}

impl Display for SelectedField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectedField::Scalar(sf) => write!(f, "{}", sf.name),
            SelectedField::Composite(cs) => write!(
                f,
                "{} {{ {} }}",
                cs.field.name,
                cs.selections
                    .iter()
                    .map(|selection| format!("{}", selection))
                    .join(", ")
            ),
        }
    }
}
