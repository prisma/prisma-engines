use itertools::Itertools;

use crate::{CompositeFieldRef, ScalarFieldRef};

#[derive(Debug)]
pub struct Index {
    pub name: Option<String>,
    pub fields: Vec<IndexField>,
    pub typ: IndexType,
}

impl Index {
    pub fn fields(&self) -> &Vec<IndexField> {
        &self.fields
    }

    pub fn scalars(&self) -> Vec<ScalarFieldRef> {
        self.fields
            .iter()
            .flat_map(|f| match f {
                IndexField::Scalar(sf) => vec![sf.clone()],
                IndexField::Composite(cif) => cif.scalars(),
            })
            .collect_vec()
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
}

#[derive(Debug, Clone)]
pub struct CompositeIndexField {
    field: CompositeFieldRef,
    nested: Vec<IndexField>,
}

impl CompositeIndexField {
    pub fn nested(&self) -> &[IndexField] {
        self.nested.as_ref()
    }

    pub fn field(&self) -> &CompositeFieldRef {
        &self.field
    }

    pub fn scalars(&self) -> Vec<ScalarFieldRef> {
        self.nested
            .iter()
            .flat_map(|f| match f {
                IndexField::Scalar(sf) => vec![sf.clone()],
                IndexField::Composite(cif) => cif.scalars(),
            })
            .collect_vec()
    }

    pub fn path(&self) -> Vec<String> {
        let mut path: Vec<String> = vec![self.field().name.clone()];

        for index_field in self.nested.iter() {
            match index_field {
                IndexField::Scalar(sf) => path.push(sf.name.clone()),
                IndexField::Composite(cif) => path.extend(cif.path()),
            }
        }

        path
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndexType {
    Unique,
    Normal,
}
