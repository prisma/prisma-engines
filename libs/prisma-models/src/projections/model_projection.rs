use super::RecordProjection;
use crate::{
    dml::FieldArity, DomainError, Field, ModelRef, PrismaValue, PrismaValueExtensions, ScalarFieldRef, TypeIdentifier,
};
use itertools::Itertools;

/// Projection of a `Model`. A projection is a (sub)set of fields of a model.
/// There can only ever be fields of one model contained in a particular `ModelProjection`
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelProjection {
    fields: Vec<Field>,
}

impl From<Field> for ModelProjection {
    fn from(f: Field) -> Self {
        Self { fields: vec![f] }
    }
}

impl ModelProjection {
    pub fn model(&self) -> ModelRef {
        self.fields[0].model()
    }

    pub fn new(fields: Vec<Field>) -> Self {
        Self {
            fields: fields.into_iter().unique_by(|f| f.name().to_owned()).collect(),
        }
    }

    /// Returns all field names (NOT database level column names!) of contained fields.
    pub fn names<'a>(&'a self) -> impl Iterator<Item = &'a str> + 'a {
        self.fields.iter().map(|field| field.name())
    }

    /// Returns all database (e.g. column or field) names of contained fields.
    pub fn db_names<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        self.scalar_fields().map(|f| f.db_name().to_owned())
    }

    /// Returns an iterator over all fields contained in this projection.
    pub fn fields<'a>(&'a self) -> impl Iterator<Item = &'a Field> + 'a {
        self.fields.iter()
    }

    /// Returns the length of scalar fields contained in this projection, e.g. the actual
    /// number of SQL columns or document fields for this model projection.
    pub fn scalar_length(&self) -> usize {
        self.scalar_fields().count()
    }

    /// Attempts to retrieve a field by name (NOT database level column name) from this projection.
    pub fn find(&self, name: &str) -> Option<&Field> {
        self.fields().find(|field| field.name() == name)
    }

    /// Returns an iterator over all scalar fields represented by this model projection, in order.
    /// Resolves relation fields to all backing scalar fields, if existing.
    pub fn scalar_fields<'a>(&'a self) -> impl Iterator<Item = ScalarFieldRef> + 'a {
        self.fields
            .iter()
            .flat_map(|field| match field {
                Field::Scalar(sf) => vec![sf.clone()],
                Field::Relation(rf) => rf.scalar_fields(),
            })
            .into_iter()
            .unique_by(|field| field.name.clone())
    }

    pub fn map_db_name(&self, name: &str) -> Option<ScalarFieldRef> {
        self.fields().find_map(|field| match field {
            Field::Scalar(sf) if sf.db_name() == name => Some(sf.clone()),
            Field::Relation(rf) => rf.scalar_fields().into_iter().find(|f| f.db_name() == name),
            _ => None,
        })
    }

    pub fn type_identifiers_with_arities(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        self.scalar_fields().map(|f| f.type_identifier_with_arity()).collect()
    }

    /// Checks if a given `RecordProjection` belongs to this `ModelProjection`.
    pub fn matches(&self, id: &RecordProjection) -> bool {
        self.scalar_fields().eq(id.fields())
    }

    /// Inserts this projections scalar fields into the given record projection.
    /// Assumes caller knows that the exchange can be done. Errors if lengths mismatch.
    /// Additionally performs a type coercion based on the source and destination field types.
    /// (Resistance is futile.)
    pub fn assimilate(&self, id: RecordProjection) -> crate::Result<RecordProjection> {
        if self.scalar_length() != id.len() {
            Err(DomainError::ConversionFailure(
                "record identifier".to_owned(),
                "assimilated record identifier".to_owned(),
            ))
        } else {
            let fields = self.scalar_fields();

            Ok(id
                .pairs
                .into_iter()
                .zip(fields)
                .map(|((og_field, value), other_field)| {
                    if og_field.type_identifier != other_field.type_identifier {
                        let value = value.coerce(&other_field.type_identifier)?;
                        Ok((other_field, value))
                    } else {
                        Ok((other_field, value))
                    }
                })
                .collect::<crate::Result<Vec<_>>>()?
                .into())
        }
    }

    /// Creates a record projection of the model projection containing only null values.
    pub fn empty_record_projection(&self) -> RecordProjection {
        self.scalar_fields()
            .map(|f| (f, PrismaValue::Null))
            .collect::<Vec<_>>()
            .into()
    }

    /// Consumes both `ModelProjection`s to create a new one that contains
    /// both fields. Each field is contained exactly once, with the first
    /// occurrence of the first field in order from left (`self`) to right (`other`)
    /// is retained. Assumes that both projections reason over the same model.
    pub fn merge(self, other: ModelProjection) -> ModelProjection {
        let fields = self.fields.into_iter().chain(other.fields).unique().collect();

        ModelProjection { fields }
    }

    /// Creates a record identifier from raw values.
    /// No checks for length, type, or similar is performed, hence "unchecked".
    pub fn from_unchecked(&self, values: Vec<PrismaValue>) -> RecordProjection {
        RecordProjection::new(self.scalar_fields().zip(values).collect())
    }

    /// Checks if this model projection contains given field.
    pub fn contains<T>(&self, field: T) -> bool
    where
        T: Into<Field>,
    {
        let field: Field = field.into();
        self.fields().any(|f| f.name() == field.name())
    }

    /// Checks if this model projection contains all the given database names.
    pub fn contains_all_db_names<'a>(&self, names: impl Iterator<Item = String>) -> bool {
        let selected_db_names: Vec<_> = self.db_names().collect();
        let names_to_select: Vec<_> = names.collect();

        if names_to_select.len() > selected_db_names.len() {
            false
        } else {
            names_to_select
                .into_iter()
                .all(|to_select| selected_db_names.contains(&to_select))
        }
    }

    /// Merges this model projection with given model projections and creates a set union of all.
    pub fn union(projections: Vec<ModelProjection>) -> ModelProjection {
        projections
            .into_iter()
            .fold(ModelProjection::default(), |acc, next| acc.merge(next))
    }
}

impl IntoIterator for ModelProjection {
    type Item = Field;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

impl From<&RecordProjection> for ModelProjection {
    fn from(p: &RecordProjection) -> Self {
        let fields = p
            .pairs
            .iter()
            .map(|(field, _)| field.clone().into())
            .collect::<Vec<_>>();

        Self::new(fields)
    }
}
