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
        Self { fields }
    }

    pub fn names<'a>(&'a self) -> impl Iterator<Item = &'a str> + 'a {
        self.fields.iter().map(|field| field.name())
    }

    pub fn db_names<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        self.scalar_fields().map(|f| f.db_name().to_owned())
    }

    pub fn fields<'a>(&'a self) -> impl Iterator<Item = &'a Field> + 'a {
        self.fields.iter()
    }

    /// Returns the length of schema model fields contained in this projection.
    /// This is **not** the length of the underlying database fields, use `db_len` instead.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns the length of scalar fields contained in this projection, e.g. the actual
    /// number of SQL columns or document fields for this model.
    pub fn db_len(&self) -> usize {
        self.scalar_fields().count()
    }

    pub fn get(&self, name: &str) -> Option<&Field> {
        self.fields().find(|field| field.name() == name)
    }

    pub fn scalar_fields<'a>(&'a self) -> impl Iterator<Item = ScalarFieldRef> + 'a {
        self.fields
            .iter()
            .flat_map(|field| match field {
                Field::Scalar(sf) => vec![sf.clone()],
                Field::Relation(rf) => rf.fields(),
            })
            .into_iter()
            .unique_by(|field| field.name.clone())
    }

    pub fn map_db_name(&self, name: &str) -> Option<ScalarFieldRef> {
        self.fields().find_map(|field| match field {
            Field::Scalar(sf) if sf.db_name() == name => Some(sf.clone()),
            Field::Relation(rf) => rf.fields().into_iter().find(|f| f.db_name() == name),
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
        if self.db_len() != id.len() {
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

    pub fn empty_record_projection(&self) -> RecordProjection {
        self.scalar_fields()
            .map(|f| (f.clone(), PrismaValue::Null))
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

    pub fn contains<T>(&self, field: T) -> bool
    where
        T: Into<Field>,
    {
        let field: Field = field.into();
        self.fields().find(|f| f.name() == field.name()).is_some()
    }

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
