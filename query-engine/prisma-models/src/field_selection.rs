use std::fmt::Display;

use itertools::Itertools;

use crate::{CompositeFieldRef, ModelRef, ScalarFieldRef};

/// A selection of fields from a model.
#[derive(Debug, Clone)]
pub struct FieldSelection {
    pub model: ModelRef,
    pub selections: Vec<SelectedField>,
}

/// A selected field. Can be contained on a model or composite type.
// Todo: Think about virtual selections like aggregations.
#[derive(Debug, Clone)]
pub enum SelectedField {
    Scalar(ScalarFieldRef),
    Composite(CompositeSelection),
}

#[derive(Debug, Clone)]
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

    /// Returns all database (e.g. column or document field) names of contained fields.
    /// Does _not_ recurse into composite selections and only checks top level fields.
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

impl From<ScalarFieldRef> for SelectedField {
    fn from(f: ScalarFieldRef) -> Self {
        Self::Scalar(f)
    }
}

impl From<(ModelRef, ScalarFieldRef)> for FieldSelection {
    fn from((model, field): (ModelRef, ScalarFieldRef)) -> Self {
        Self {
            model,
            selections: vec![field.into()],
        }
    }
}

impl Display for FieldSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FieldSelection {{ model: '{}', fields: [{}] }}",
            self.model.name,
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
