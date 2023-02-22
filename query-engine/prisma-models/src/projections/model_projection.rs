use crate::{dml::FieldArity, Field, FieldSelection, ScalarFieldRef, SelectedField, SelectionResult, TypeIdentifier};
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

impl From<FieldSelection> for ModelProjection {
    fn from(fs: FieldSelection) -> Self {
        (&fs).into()
    }
}

/// [Composites] todo: Temporary converter.
impl From<&FieldSelection> for ModelProjection {
    fn from(fs: &FieldSelection) -> Self {
        Self {
            fields: fs
                .selections()
                .filter_map(|selected| match selected {
                    SelectedField::Scalar(sf) => Some(sf.clone().into()),
                    SelectedField::Composite(_cf) => None,
                })
                .collect(),
        }
    }
}

impl ModelProjection {
    pub fn new(fields: Vec<Field>) -> Self {
        Self {
            fields: fields.into_iter().unique_by(|f| f.name().to_owned()).collect(),
        }
    }

    pub fn new_from_scalar(fields: Vec<ScalarFieldRef>) -> Self {
        Self::new(fields.into_iter().map(Field::Scalar).collect())
    }

    /// Returns all field names (NOT database level column names!) of contained fields.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|field| field.name())
    }

    /// Returns all database (e.g. column or field) names of contained fields.
    pub fn db_names(&self) -> impl Iterator<Item = String> + '_ {
        self.scalar_fields().map(|f| f.db_name().to_owned())
    }

    /// Returns an iterator over all fields contained in this projection.
    pub fn fields(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter()
    }

    /// Returns the length of scalar fields contained in this projection, e.g. the actual
    /// number of SQL columns or document fields for this model projection.
    pub fn scalar_length(&self) -> usize {
        self.scalar_fields().count()
    }

    /// Returns an iterator over all scalar fields represented by this model projection, in order.
    /// Resolves relation fields to all backing scalar fields, if existing.
    pub fn scalar_fields(&self) -> impl Iterator<Item = ScalarFieldRef> + '_ {
        self.fields
            .iter()
            .flat_map(|field| match field {
                Field::Scalar(sf) => vec![sf.clone()],
                Field::Relation(rf) => rf.scalar_fields(),
                Field::Composite(_) => todo!(), // [Composites] todo
            })
            .into_iter()
            .unique_by(|field| field.name().to_owned())
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
}

impl IntoIterator for ModelProjection {
    type Item = Field;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

impl From<&SelectionResult> for ModelProjection {
    fn from(p: &SelectionResult) -> Self {
        let fields = p
            .pairs
            .iter()
            .map(|(field_selection, _)| match field_selection {
                SelectedField::Scalar(sf) => sf.clone().into(),
                SelectedField::Composite(cf) => cf.field.clone().into(),
            })
            .collect::<Vec<_>>();

        Self::new(fields)
    }
}
