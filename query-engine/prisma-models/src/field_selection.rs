use crate::{
    parent_container::ParentContainer, CompositeFieldRef, DomainError, Field, PrismaValueExtensions, ScalarFieldRef,
    SelectionResult,
};
use itertools::Itertools;
use prisma_value::PrismaValue;
use std::fmt::Display;

/// A selection of fields from a model.
#[derive(Debug, Clone, PartialEq, Default, Hash, Eq)]
pub struct FieldSelection {
    selections: Vec<SelectedField>,
}

impl FieldSelection {
    pub fn new(selections: Vec<SelectedField>) -> Self {
        Self { selections }
    }

    pub fn into_inner(self) -> Vec<SelectedField> {
        self.selections
    }

    /// Returns `true` if self contains (at least) all fields specified in `other`. `false` otherwise.
    /// Recurses into composite selections and ensures that composite selections are supersets as well.
    pub fn is_superset_of(&self, other: &Self) -> bool {
        other.selections.iter().all(|selection| match selection {
            SelectedField::Scalar(sf) => self.contains(sf.name()),
            SelectedField::Composite(other_cs) => self
                .get(other_cs.field.name())
                .and_then(|selection| selection.as_composite())
                .map(|cs| cs.is_superset_of(other_cs))
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

    /// Inserts this `FieldSelection`s selections into the given `SelectionResult`.
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
            let pairs = values
                .pairs
                .into_iter()
                .zip(self.selections())
                .map(|((_, value), new_selection)| {
                    let value = new_selection.coerce_value(value)?;
                    Ok((new_selection.clone(), value))
                })
                .collect::<crate::Result<Vec<_>>>()?;

            Ok(SelectionResult::new(pairs))
        }
    }

    /// Checks if a given `SelectionResult` belongs to this `FieldSelection`.
    pub fn matches(&self, result: &SelectionResult) -> bool {
        result.pairs.iter().all(|(rt, _)| self.selections.contains(rt))
    }

    /// Merges all given `FieldSelection` a set union of all.
    /// Each selection is contained exactly once, with the first
    /// occurrence of the first field in order from left to right
    /// is retained.
    ///
    /// /!\ Important assumption: All selections are on the same model.
    pub fn union(selections: Vec<Self>) -> Self {
        let chained = selections.into_iter().flatten();

        FieldSelection {
            selections: chained.unique().collect(),
        }
    }

    /// Consumes both `FieldSelection`s to create a new one that contains a union
    /// of both. Each selection is contained exactly once, with the first
    /// occurrence of the first field in order from left (`self`) to right (`other`)
    /// is retained. Assumes that both selections reason over the same model.
    pub fn merge(self, other: FieldSelection) -> FieldSelection {
        let selections = self.selections.into_iter().chain(other.selections).unique().collect();

        FieldSelection { selections }
    }
}

/// A selected field. Can be contained on a model or composite type.
// Todo: Think about virtual selections like aggregations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SelectedField {
    Scalar(ScalarFieldRef),
    Composite(CompositeSelection),
}

impl SelectedField {
    pub fn prisma_name(&self) -> &str {
        match self {
            SelectedField::Scalar(sf) => sf.name(),
            SelectedField::Composite(cf) => cf.field.name(),
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

    pub fn container(&self) -> ParentContainer {
        match self {
            SelectedField::Scalar(sf) => sf.container(),
            SelectedField::Composite(cs) => cs.field.container(),
        }
    }

    /// Coerces a value to fit the selection. If the conversion is not possible, an error will be thrown.
    pub fn coerce_value(&self, value: PrismaValue) -> crate::Result<PrismaValue> {
        match self {
            SelectedField::Scalar(sf) => value.coerce(&sf.type_identifier()),
            SelectedField::Composite(cs) => cs.coerce_value(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompositeSelection {
    pub field: CompositeFieldRef,
    pub selections: Vec<SelectedField>,
}

impl CompositeSelection {
    pub fn is_superset_of(&self, other: &Self) -> bool {
        self.field.typ() == other.field.typ()
            && other.selections.iter().all(|selection| match selection {
                SelectedField::Scalar(sf) => self.contains(sf.name()),
                SelectedField::Composite(other_cs) => self
                    .get(other_cs.field.name())
                    .and_then(|selection| selection.as_composite())
                    .map(|cs| cs.is_superset_of(other_cs))
                    .unwrap_or(false),
            })
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn get(&self, name: &str) -> Option<&SelectedField> {
        self.selections.iter().find(|selection| selection.prisma_name() == name)
    }

    /// Ensures that the given `PrismaValue` fits this composite selection. That includes:
    /// - Discarding extra fields on objects.
    /// - All scalar leafs are coerced to their type ident.
    fn coerce_value(&self, pv: PrismaValue) -> crate::Result<PrismaValue> {
        match pv {
            PrismaValue::Object(pairs) => Ok(PrismaValue::Object(
                pairs
                    .into_iter()
                    .map(|(key, value)| match self.get(&key) {
                        Some(selection) => Ok((key, selection.coerce_value(value)?)),
                        None => Err(DomainError::FieldNotFound {
                            name: key.clone(),
                            container_name: self.field.name().to_owned(),
                            container_type: "composite type",
                        }),
                    })
                    .collect::<crate::Result<Vec<_>>>()?,
            )),

            val => Err(DomainError::ConversionFailure(
                val.to_string(),
                "Prisma object value".to_owned(),
            )),
        }
    }
}

impl From<Vec<ScalarFieldRef>> for FieldSelection {
    fn from(fields: Vec<ScalarFieldRef>) -> Self {
        Self {
            selections: fields.into_iter().map(Into::into).collect(),
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
                .map(|selection| format!("{selection}"))
                .join(", ")
        )
    }
}

impl Display for SelectedField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectedField::Scalar(sf) => write!(f, "{sf}"),
            SelectedField::Composite(cs) => write!(
                f,
                "{} {{ {} }}",
                cs.field,
                cs.selections.iter().map(|selection| format!("{selection}")).join(", ")
            ),
        }
    }
}

impl From<&SelectionResult> for FieldSelection {
    fn from(p: &SelectionResult) -> Self {
        let selections = p
            .pairs
            .iter()
            .map(|(selected_field, _)| selected_field.clone())
            .collect::<Vec<_>>();

        Self { selections }
    }
}

impl IntoIterator for FieldSelection {
    type Item = SelectedField;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.selections.into_iter()
    }
}
