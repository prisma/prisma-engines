use itertools::Itertools;
use std::collections::HashSet;

use crate::{CompositeFieldRef, CompositeTypeRef, Field, ScalarFieldRef};

#[derive(Debug, Clone)]
pub struct Index {
    pub name: Option<String>,
    pub fields: Vec<IndexField>,
    pub typ: IndexType,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndexType {
    Unique,
    Normal,
}

impl Index {
    pub fn fields(&self) -> &[IndexField] {
        &self.fields
    }

    pub fn scalars(&self) -> Vec<&ScalarFieldRef> {
        self.fields.iter().flat_map(|f| f.scalars()).collect_vec()
    }
}

#[derive(Debug, Clone)]
pub enum IndexField {
    Scalar(ScalarFieldRef),
    Composite(CompositeIndexField),
}

impl IndexField {
    pub fn scalar(sf: ScalarFieldRef) -> Self {
        IndexField::Scalar(sf)
    }

    pub fn composite(field: CompositeFieldRef, nested: Vec<IndexField>) -> Self {
        IndexField::Composite(CompositeIndexField { field, nested })
    }

    pub fn from_scalars(fields: Vec<ScalarFieldRef>) -> Vec<IndexField> {
        fields.into_iter().map(Self::scalar).collect_vec()
    }

    pub fn path(&self) -> Vec<String> {
        match self {
            IndexField::Scalar(sf) => vec![sf.name.clone()],
            IndexField::Composite(cif) => cif.path(),
        }
    }

    pub fn scalars(&self) -> Vec<&ScalarFieldRef> {
        match self {
            IndexField::Scalar(sf) => vec![sf],
            IndexField::Composite(cif) => cif.scalars(),
        }
    }

    pub fn as_composite(&self) -> Option<&CompositeIndexField> {
        match self {
            IndexField::Composite(cf) => Some(cf),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompositeIndexField {
    field: CompositeFieldRef,
    nested: Vec<IndexField>,
}

impl CompositeIndexField {
    pub fn field(&self) -> &CompositeFieldRef {
        &self.field
    }

    /// Return true if the index targets individual fields of the composite type
    pub fn is_partial(&self) -> bool {
        !self.nested.is_empty()
    }

    pub fn index_fields(&self) -> Vec<IndexField> {
        if self.is_partial() {
            self.nested.clone()
        } else {
            // If the index is not partial, then compute all the index fields
            self.field
                .typ
                .fields()
                .iter()
                .map(|field| match field {
                    Field::Scalar(sf) => IndexField::scalar(sf.clone()),
                    Field::Composite(cf) => IndexField::composite(cf.clone(), vec![]),
                    Field::Relation(_) => unreachable!(),
                })
                .collect_vec()
        }
    }

    pub fn scalars(&self) -> Vec<&ScalarFieldRef> {
        if self.is_partial() {
            self.nested.iter().flat_map(|f| f.scalars()).collect_vec()
        } else {
            collect_all_composite_scalar_fields(&self.field.typ, &mut HashSet::default())
        }
    }

    pub fn path(&self) -> Vec<String> {
        let mut path: Vec<String> = vec![self.field().name.clone()];

        for index_field in &self.nested {
            match index_field {
                IndexField::Scalar(sf) => path.push(sf.name.clone()),
                IndexField::Composite(cif) => path.extend(cif.path()),
            }
        }

        path
    }
}

impl From<CompositeFieldRef> for CompositeIndexField {
    fn from(field: CompositeFieldRef) -> Self {
        Self { field, nested: vec![] }
    }
}

/// Recursively collect all scalar fields of a CompositeType.
pub fn collect_all_composite_scalar_fields<'a>(
    typ: &'a CompositeTypeRef,
    visited: &mut HashSet<String>,
) -> Vec<&'a ScalarFieldRef> {
    let mut scalars = vec![];

    for field in typ.fields() {
        match field {
            Field::Scalar(sf) => {
                scalars.push(sf);
            }
            Field::Composite(cf) => {
                // Safe because names are unique in the datamodel
                if !visited.contains(&cf.typ.name) {
                    visited.insert(cf.typ.name.clone());

                    scalars.extend(collect_all_composite_scalar_fields(&cf.typ, visited));
                }
            }
            Field::Relation(_) => unreachable!(),
        }
    }

    scalars
}
