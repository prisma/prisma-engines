use crate::{
    parent_container::ParentContainer, prisma_value_ext::PrismaValueExtensions, CompositeFieldRef, DomainError, Field,
    Filter, Model, ModelProjection, QueryArguments, RelationField, RelationFieldRef, ScalarField, ScalarFieldRef,
    SelectionResult, Type, TypeIdentifier,
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

    pub fn selections(&self) -> impl ExactSizeIterator<Item = &SelectedField> + '_ {
        self.selections.iter()
    }

    pub fn scalars(&self) -> impl Iterator<Item = &ScalarFieldRef> + '_ {
        self.selections().filter_map(SelectedField::as_scalar)
    }

    pub fn virtuals(&self) -> impl Iterator<Item = &VirtualSelection> {
        self.selections().filter_map(SelectedField::as_virtual)
    }

    pub fn virtuals_owned(&self) -> Vec<VirtualSelection> {
        self.virtuals().cloned().collect()
    }

    pub fn without_relations(&self) -> Self {
        FieldSelection::new(
            self.selections()
                .filter(|field| !matches!(field, SelectedField::Relation(_)))
                .cloned()
                .collect(),
        )
    }

    pub fn into_virtuals_last(self) -> Self {
        let (virtuals, non_virtuals): (Vec<_>, Vec<_>) = self
            .into_iter()
            .partition(|field| matches!(field, SelectedField::Virtual(_)));

        FieldSelection::new(non_virtuals.into_iter().chain(virtuals).collect())
    }

    pub fn to_virtuals_last(&self) -> Self {
        self.clone().into_virtuals_last()
    }

    /// Returns the selections, grouping the virtual fields that are wrapped into objects in the
    /// query (like `_count`) and returning only the first virtual field in each of those groups.
    /// This is useful when we want to treat the group as a whole but we don't need the information
    /// about every field in the group and can infer the necessary information (like the group
    /// name) from any of those fields. This method is used by
    /// [`FieldSelection::db_names_grouping_virtuals`] and
    /// [`FieldSelection::type_identifiers_with_arities_grouping_virtuals`].
    fn selections_with_virtual_group_heads(&self) -> impl Iterator<Item = &SelectedField> {
        self.selections().unique_by(|f| f.prisma_name_grouping_virtuals())
    }

    /// Returns all Prisma (e.g. schema model field) names of contained fields.
    /// Does _not_ recurse into composite selections and only iterates top level fields.
    pub fn prisma_names(&self) -> impl Iterator<Item = String> + '_ {
        self.selections().map(|f| f.prisma_name().into_owned())
    }

    /// Returns all database (e.g. column or document field) names of contained fields.
    /// Does _not_ recurse into composite selections and only iterates top level fields.
    /// Returns db aliases for virtual fields grouped into objects in the query separately,
    /// representing results of queries that do not load relations using JOINs.
    pub fn db_names(&self) -> impl Iterator<Item = String> + '_ {
        self.selections().map(|f| f.db_name().into_owned())
    }

    /// Returns all database (e.g. column or document field) names of contained fields. Does not
    /// recurse into composite selections and only iterates top level fields. Also does not recurse
    /// into the grouped containers for virtual fields, like `_count`. The names returned by this
    /// method correspond to the results of queries that use JSON objects to represent joined
    /// relations and relation aggregations.
    pub fn prisma_names_grouping_virtuals(&self) -> impl Iterator<Item = String> + '_ {
        self.selections_with_virtual_group_heads()
            .map(|f| f.prisma_name_grouping_virtuals())
            .map(Cow::into_owned)
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
                _ => None,
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

    /// Returns type identifiers and arities, treating all virtual fields as separate fields.
    pub fn type_identifiers_with_arities(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        self.selections()
            .filter_map(SelectedField::type_identifier_with_arity)
            .collect()
    }

    /// Returns type identifiers and arities, grouping the virtual fields so that the type
    /// identifier and arity is returned for the whole object containing multiple virtual fields
    /// and not each of those fields separately. This represents the selection in joined queries
    /// that use JSON objects for relations and relation aggregations.
    pub fn type_identifiers_with_arities_grouping_virtuals(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        self.selections_with_virtual_group_heads()
            .filter_map(|vs| vs.type_identifier_with_arity_grouping_virtuals())
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

impl AsRef<[SelectedField]> for FieldSelection {
    fn as_ref(&self) -> &[SelectedField] {
        &self.selections
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

    pub fn virtuals(&self) -> impl Iterator<Item = &VirtualSelection> {
        self.selections.iter().filter_map(SelectedField::as_virtual)
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

    pub fn serialized_name(&self) -> (&'static str, &str) {
        (self.serialized_group_name(), self.serialized_field_name())
    }

    pub fn serialized_group_name(&self) -> &'static str {
        match self {
            // TODO: we can't use UNDERSCORE_COUNT here because it would require a circular
            // dependency between `schema` and `query-structure` crates.
            Self::RelationCount(_, _) => "_count",
        }
    }

    pub fn serialized_field_name(&self) -> &str {
        match self {
            Self::RelationCount(rf, _) => rf.name(),
        }
    }

    pub fn model(&self) -> Model {
        match self {
            Self::RelationCount(rf, _) => rf.model(),
        }
    }

    pub fn coerce_value(&self, value: PrismaValue) -> crate::Result<PrismaValue> {
        match self {
            Self::RelationCount(rf, _) => match value {
                PrismaValue::Null => Ok(PrismaValue::Int(0)),
                _ => value.coerce(&rf.zip(TypeIdentifier::Int)),
            },
        }
    }

    pub fn field(&self) -> Field {
        match self {
            Self::RelationCount(rf, _) => rf.clone().into(),
        }
    }

    pub fn type_identifier(&self) -> TypeIdentifier {
        match self {
            Self::RelationCount(_, _) => TypeIdentifier::Int,
        }
    }

    pub fn arity(&self) -> FieldArity {
        match self {
            Self::RelationCount(_, _) => FieldArity::Required,
        }
    }

    pub fn type_identifier_with_arity(&self) -> (TypeIdentifier, FieldArity) {
        (self.type_identifier(), self.arity())
    }

    pub fn r#type(&self) -> Type {
        match self {
            Self::RelationCount(rf, _) => rf.zip(self.type_identifier()),
        }
    }

    pub fn relation_field(&self) -> &RelationField {
        match self {
            VirtualSelection::RelationCount(rf, _) => rf,
        }
    }

    pub fn filter(&self) -> Option<&Filter> {
        match self {
            VirtualSelection::RelationCount(_, filter) => filter.as_ref(),
        }
    }
}

impl Display for VirtualSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let model = self.relation_field().model();
        let model_name = model.name();
        let (obj, field) = self.serialized_name();

        write!(f, "{model_name}.{obj}.{field}")
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

    /// Returns the name of the field in the database (if applicable) or other kind of name that is
    /// used in the queries for this field. For virtual fields, this returns the alias used in the
    /// queries that do not group them into objects.
    pub fn db_name(&self) -> Cow<'_, str> {
        match self {
            SelectedField::Scalar(sf) => sf.db_name().into(),
            SelectedField::Composite(cs) => cs.field.db_name().into(),
            SelectedField::Relation(rs) => rs.field.name().into(),
            SelectedField::Virtual(vs) => vs.db_alias().into(),
        }
    }

    /// Returns the name of the field in the database (if applicable) or other kind of name that is
    /// used in the queries for this field. For virtual fields that are wrapped inside an object in
    /// Prisma queries, this returns the name of the surrounding object and not the field itself,
    /// so this method can return identical values for multiple fields in the [`FieldSelection`].
    /// This is used in queries with relation JOINs which use JSON objects to represent both
    /// relations and relation aggregations. For those queries, the result of this method
    /// corresponds to the top-level name of the value which is a JSON object that contains this
    /// field inside.
    pub fn prisma_name_grouping_virtuals(&self) -> Cow<'_, str> {
        match self {
            SelectedField::Virtual(vs) => vs.serialized_name().0.into(),
            _ => self.prisma_name(),
        }
    }

    /// Returns the type identifier and arity of this field, unless it is a composite field, in
    /// which case [`None`] is returned.
    pub fn type_identifier_with_arity(&self) -> Option<(TypeIdentifier, FieldArity)> {
        match self {
            SelectedField::Scalar(sf) => Some(sf.type_identifier_with_arity()),
            SelectedField::Relation(rf) if rf.field.is_list() => Some((TypeIdentifier::Json, FieldArity::Required)),
            SelectedField::Relation(rf) => Some((TypeIdentifier::Json, rf.field.arity())),
            SelectedField::Composite(_) => None,
            SelectedField::Virtual(vs) => Some(vs.type_identifier_with_arity()),
        }
    }

    /// Returns the type identifier and arity of this field, unless it is a composite field, in
    /// which case [`None`] is returned.
    ///
    /// In the case of virtual fields that are wrapped into objects in Prisma queries
    /// (specifically, relation aggregations), the returned information refers not to the current
    /// field itself but to the whole object that contains this field. This is used by the queries
    /// with relation JOINs because they use JSON objects to reprsent both relations and relation
    /// aggregations, so individual virtual fields that correspond to those relation aggregations
    /// don't exist as separate values in the result of the query.
    pub fn type_identifier_with_arity_grouping_virtuals(&self) -> Option<(TypeIdentifier, FieldArity)> {
        match self {
            SelectedField::Virtual(_) => Some((TypeIdentifier::Json, FieldArity::Required)),
            _ => self.type_identifier_with_arity(),
        }
    }

    pub fn as_scalar(&self) -> Option<&ScalarFieldRef> {
        match self {
            SelectedField::Scalar(sf) => Some(sf),
            _ => None,
        }
    }

    pub fn as_composite(&self) -> Option<&CompositeSelection> {
        match self {
            SelectedField::Composite(ref cs) => Some(cs),
            _ => None,
        }
    }

    pub fn as_virtual(&self) -> Option<&VirtualSelection> {
        match self {
            SelectedField::Virtual(vs) => Some(vs),
            _ => None,
        }
    }

    pub fn container(&self) -> ParentContainer {
        match self {
            SelectedField::Scalar(sf) => sf.container(),
            SelectedField::Composite(cs) => cs.field.container(),
            SelectedField::Relation(rs) => ParentContainer::from(rs.field.model()),
            SelectedField::Virtual(vs) => ParentContainer::from(vs.model()),
        }
    }

    /// Coerces a value to fit the selection. If the conversion is not possible, an error will be thrown.
    pub(crate) fn coerce_value(&self, value: PrismaValue) -> crate::Result<PrismaValue> {
        match self {
            SelectedField::Scalar(sf) => value.coerce(&sf.r#type()),
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
                SelectedField::Virtual(vs) => self.contains(&vs.db_alias()),
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

impl<T: IntoIterator<Item = ScalarFieldRef>> From<T> for FieldSelection {
    fn from(fields: T) -> Self {
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
