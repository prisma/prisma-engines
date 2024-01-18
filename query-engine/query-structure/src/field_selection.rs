use crate::{
    parent_container::ParentContainer, prisma_value_ext::PrismaValueExtensions, CompositeFieldRef, DomainError, Field,
    Filter, Model, ModelProjection, QueryArguments, RelationField, RelationFieldRef, ScalarField, ScalarFieldRef,
    SelectionResult, TypeIdentifier,
};
use itertools::Itertools;
use prisma_value::PrismaValue;
use psl::schema_ast::ast::FieldArity;
use std::{borrow::Cow, fmt::Display};

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
            // TODO: Relation selections are ignored for now to prevent breaking the existing query-based strategy to resolve relations.
            SelectedField::Relation(_) => true,
            SelectedField::Virtual(vs) => self.contains(&vs.db_alias()),
        })
    }

    pub fn selections(&self) -> impl Iterator<Item = &SelectedField> + '_ {
        self.selections.iter()
    }

    pub fn virtuals(&self) -> impl Iterator<Item = &VirtualSelection> {
        self.selections().filter_map(|field| match field {
            SelectedField::Virtual(ref vs) => Some(vs),
            _ => None,
        })
    }

    /// Returns all Prisma (e.g. schema model field) names of contained fields.
    /// Does _not_ recurse into composite selections and only iterates top level fields.
    pub fn prisma_names(&self) -> impl Iterator<Item = String> + '_ {
        self.selections.iter().map(|f| f.prisma_name().into_owned())
    }

    /// Returns all database (e.g. column or document field) names of contained fields.
    /// Does _not_ recurse into composite selections and only iterates level fields.
    pub fn db_names(&self) -> impl Iterator<Item = String> + '_ {
        self.selections.iter().map(|f| f.db_name().into_owned())
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
                SelectedField::Relation(rs) => rs.field.clone().into(),
                SelectedField::Virtual(vs) => vs.field(),
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
                SelectedField::Relation(_) => None,
                SelectedField::Virtual(_) => None,
            })
            .collect::<Vec<_>>();

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

    pub fn merge_in_place(&mut self, other: FieldSelection) {
        let this = std::mem::take(self);
        *self = this.merge(other);
    }

    pub fn type_identifiers_with_arities(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        self.selections()
            .filter_map(|selection| match selection {
                SelectedField::Scalar(sf) => Some(sf.type_identifier_with_arity()),
                SelectedField::Relation(rf) if rf.field.is_list() => Some((TypeIdentifier::Json, FieldArity::Required)),
                SelectedField::Relation(rf) => Some((TypeIdentifier::Json, rf.field.arity())),
                SelectedField::Composite(_) => None,
                SelectedField::Virtual(vs) => Some(vs.type_identifier_with_arity()),
            })
            .collect()
    }

    pub fn relations(&self) -> impl Iterator<Item = &RelationSelection> {
        self.selections().filter_map(|selection| match selection {
            SelectedField::Relation(rs) => Some(rs),
            _ => None,
        })
    }

    pub fn into_projection(self) -> ModelProjection {
        self.into()
    }

    pub fn has_virtual_fields(&self) -> bool {
        self.selections()
            .any(|field| matches!(field, SelectedField::Virtual(_)))
    }
}

/// A selected field. Can be contained on a model or composite type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SelectedField {
    Scalar(ScalarFieldRef),
    Composite(CompositeSelection),
    Relation(RelationSelection),
    Virtual(VirtualSelection),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelationSelection {
    pub field: RelationField,
    pub args: QueryArguments,
    /// Field names that will eventually be serialized
    pub result_fields: Vec<String>,
    // Fields that will be queried by the connectors
    pub selections: Vec<SelectedField>,
}

impl RelationSelection {
    pub fn scalars(&self) -> impl Iterator<Item = &ScalarField> {
        self.selections.iter().filter_map(|selection| match selection {
            SelectedField::Scalar(sf) => Some(sf),
            _ => None,
        })
    }

    pub fn relations(&self) -> impl Iterator<Item = &RelationSelection> {
        self.selections.iter().filter_map(|selection| match selection {
            SelectedField::Relation(rs) => Some(rs),
            _ => None,
        })
    }

    pub fn related_model(&self) -> Model {
        self.field.related_model()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VirtualSelection {
    RelationCount(RelationFieldRef, Option<Filter>),
}

impl VirtualSelection {
    pub fn db_alias(&self) -> String {
        match self {
            Self::RelationCount(rf, _) => format!("_aggr_count_{}", rf.name()),
        }
    }

    pub fn model(&self) -> Model {
        match self {
            Self::RelationCount(rf, _) => rf.model(),
        }
    }

    pub fn coerce_value(&self, value: PrismaValue) -> crate::Result<PrismaValue> {
        match self {
            Self::RelationCount(_, _) => match value {
                PrismaValue::Null => Ok(PrismaValue::Int(0)),
                _ => value.coerce(TypeIdentifier::Int),
            },
        }
    }

    pub fn field(&self) -> Field {
        match self {
            Self::RelationCount(rf, _) => rf.clone().into(),
        }
    }

    pub fn type_identifier_with_arity(&self) -> (TypeIdentifier, FieldArity) {
        match self {
            Self::RelationCount(_, _) => (TypeIdentifier::Int, FieldArity::Required),
        }
    }
}

impl Display for VirtualSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.db_alias())
    }
}

impl SelectedField {
    pub fn prisma_name(&self) -> Cow<'_, str> {
        match self {
            SelectedField::Scalar(sf) => sf.name().into(),
            SelectedField::Composite(cf) => cf.field.name().into(),
            SelectedField::Relation(rs) => rs.field.name().into(),
            SelectedField::Virtual(vs) => vs.db_alias().into(),
        }
    }

    pub fn db_name(&self) -> Cow<'_, str> {
        match self {
            SelectedField::Scalar(sf) => sf.db_name().into(),
            SelectedField::Composite(cs) => cs.field.db_name().into(),
            SelectedField::Relation(rs) => rs.field.name().into(),
            SelectedField::Virtual(vs) => vs.db_alias().into(),
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
            SelectedField::Relation(rs) => ParentContainer::from(rs.field.model()),
            SelectedField::Virtual(vs) => ParentContainer::from(vs.model()), // TODO
        }
    }

    /// Coerces a value to fit the selection. If the conversion is not possible, an error will be thrown.
    pub(crate) fn coerce_value(&self, value: PrismaValue) -> crate::Result<PrismaValue> {
        match self {
            SelectedField::Scalar(sf) => value.coerce(sf.type_identifier()),
            SelectedField::Composite(cs) => cs.coerce_value(value),
            SelectedField::Relation(_) => todo!(),
            SelectedField::Virtual(vs) => vs.coerce_value(value),
        }
    }

    /// Returns `true` if the selected field is [`Scalar`].
    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar(..))
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
                SelectedField::Relation(_) => true, // A composite selection cannot hold relations.
                SelectedField::Virtual(_) => true,  // TODO
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
            SelectedField::Relation(rs) => write!(
                f,
                "{} {{ {} }}",
                rs.field,
                rs.selections.iter().map(|selection| format!("{selection}")).join(", ")
            ),
            SelectedField::Virtual(vs) => write!(f, "{vs}"),
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
