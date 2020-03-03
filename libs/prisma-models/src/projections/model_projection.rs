use super::RecordProjection;
use crate::{
    dml::FieldArity, DataSourceFieldRef, DomainError, Field, ModelRef, PrismaValue, PrismaValueExtensions,
    TypeIdentifier,
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
        self.data_source_fields().map(|dsf| dsf.name.clone())
    }

    pub fn fields<'a>(&'a self) -> impl Iterator<Item = &'a Field> + 'a {
        self.fields.iter()
    }

    /// Returns the length of schema model fields contained in this projection.
    /// This is **not** the length of the underlying database fields, use `db_len` instead.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns the length of data source fields contained in this projection, e.g. the actual
    /// number of SQL columns or document fields for this model.
    pub fn db_len(&self) -> usize {
        self.data_source_fields().count()
    }

    pub fn get(&self, name: &str) -> Option<&Field> {
        self.fields().find(|field| field.name() == name)
    }

    // [DTODO] Hack to ignore m2m fields, remove when no dsfs are set on m2m rels anymore.
    pub fn data_source_fields<'a>(&'a self) -> impl Iterator<Item = DataSourceFieldRef> + 'a {
        self.fields
            .iter()
            .flat_map(|field| match field {
                Field::Scalar(sf) => vec![sf.data_source_field().clone()],
                Field::Relation(rf) if rf.relation().is_many_to_many() => vec![],
                Field::Relation(rf) => rf.data_source_fields().to_vec(),
            })
            .into_iter()
    }

    pub fn map_db_name(&self, name: &str) -> Option<&DataSourceFieldRef> {
        self.fields().find_map(|field| match field {
            Field::Scalar(sf) if sf.data_source_field().name == name => Some(sf.data_source_field()),
            Field::Relation(rf) => rf.data_source_fields().iter().find(|dsf| dsf.name == name),
            _ => None,
        })
    }

    pub fn type_identifiers_with_arities(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        self.data_source_fields()
            .map(|dsf| (dsf.field_type.into(), dsf.arity))
            .collect()
    }

    /// Checks if a given `RecordProjection` belongs to this `ModelProjection`.
    pub fn matches(&self, id: &RecordProjection) -> bool {
        self.data_source_fields().eq(id.fields())
    }

    /// Inserts this projections data source fields into the given record projection.
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
            let fields = self.data_source_fields();

            Ok(id
                .pairs
                .into_iter()
                .zip(fields)
                .map(|((og_field, value), other_field)| {
                    if og_field.field_type != other_field.field_type {
                        let coerce_to: TypeIdentifier = other_field.field_type.into();
                        Ok((other_field, value.coerce(&coerce_to)?))
                    } else {
                        Ok((other_field, value))
                    }
                })
                .collect::<crate::Result<Vec<_>>>()?
                .into())
        }
    }

    pub fn empty_record_projection(&self) -> RecordProjection {
        self.data_source_fields()
            .map(|dsf| (dsf.clone(), PrismaValue::Null))
            .collect::<Vec<_>>()
            .into()
    }

    /// Consumes both `ModelProjection`s to create a new one that contains
    /// both fields. Each field is contained exactly once, with the first
    /// occurrence of the first field in order from left (`self`) to right (`other`)
    /// is retained. Assumes that both projections reason over the same model.
    pub fn merge(self, other: ModelProjection) -> ModelProjection {
        assert_eq!(self.model(), other.model());
        let fields = self.fields.into_iter().chain(other.fields).unique().collect();

        ModelProjection { fields }
    }

    /// Creates a record identifier from raw values.
    /// No checks for length, type, or similar is performed, hence "unchecked".
    pub fn from_unchecked(&self, values: Vec<PrismaValue>) -> RecordProjection {
        RecordProjection::new(self.data_source_fields().zip(values).collect())
    }
}

impl IntoIterator for ModelProjection {
    type Item = Field;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}
