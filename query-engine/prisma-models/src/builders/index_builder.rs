use std::sync::Arc;

use crate::{Field, Index, IndexType, ScalarFieldRef, ScalarFieldWeak};

#[derive(Debug)]
pub struct IndexBuilder {
    pub name: Option<String>,
    pub field_paths: Vec<Vec<String>>,
    pub typ: IndexType,
}

impl IndexBuilder {
    pub fn build(self, all_fields: &[Field]) -> Index {
        let fields = match self.typ {
            IndexType::Unique => Self::map_fields(self.field_paths, all_fields),
            IndexType::Normal => Self::map_fields(self.field_paths, all_fields),
        };

        Index {
            name: self.name,
            typ: self.typ,
            fields,
        }
    }

    fn map_fields(field_paths: Vec<Vec<String>>, all_fields: &[Field]) -> Vec<(Vec<String>, ScalarFieldWeak)> {
        field_paths
            .into_iter()
            .map(|path| {
                let field = if path.len() == 1 {
                    let name = path.first().unwrap();
                    all_fields
                        .into_iter()
                        .find(|&f| f.name() == name)
                        .map(|f| f.clone())
                        .and_then(|f| f.into_scalar())
                } else {
                    find_scalar_in_composite_fields(path.as_slice(), &all_fields)
                }
                .unwrap_or_else(|| panic!("Unable to resolve field path '{}'", path.join(".")));

                (path, Arc::downgrade(&field))
            })
            .collect()
    }
}

fn find_scalar_in_composite_fields(path: &[String], fields: &[Field]) -> Option<ScalarFieldRef> {
    // Recursively go through the embedded fields until finding the scalar
    let name = path.first();
    if let Some(field) = fields.iter().find(|f| f.name() == name.unwrap()) {
        match (path, field) {
            ([_], Field::Composite(_)) => None,
            ([_], Field::Scalar(field_ref)) => Some(field_ref.clone()),
            (_, Field::Composite(field_ref)) => find_scalar_in_composite_fields(&path[1..], &field_ref.typ.fields()),
            (_, _) => None,
        }
    } else {
        None
    }
}
