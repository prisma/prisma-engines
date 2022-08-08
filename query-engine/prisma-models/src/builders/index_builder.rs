use crate::{Field, Index, IndexField, IndexType};

use itertools::Itertools;

type FieldPaths = Vec<Vec<String>>;

#[derive(Debug)]
pub struct IndexBuilder {
    pub name: Option<String>,
    pub field_paths: FieldPaths,
    pub typ: IndexType,
}

impl IndexBuilder {
    pub fn build(self, all_fields: &[Field]) -> Index {
        let fields = match self.typ {
            IndexType::Unique => Self::build_index_fields(self.field_paths, all_fields),
            IndexType::Normal => Self::build_index_fields(self.field_paths, all_fields),
        };

        Index {
            name: self.name,
            typ: self.typ,
            fields,
        }
    }

    fn build_index_fields(field_paths: FieldPaths, fields: &[Field]) -> Vec<IndexField> {
        let mut index_fields: Vec<IndexField> = vec![];

        for (field_name, grouped_paths) in &field_paths.into_iter().group_by(|path| path.first().cloned().unwrap()) {
            let grouped_paths = grouped_paths.collect_vec();
            let field = fields.iter().find(|f| f.name() == field_name);

            match field {
                Some(Field::Composite(cf)) => {
                    let walked_paths = walk_composite_paths(grouped_paths);
                    let nested = Self::build_index_fields(walked_paths, cf.typ.fields());

                    index_fields.push(IndexField::composite(cf.clone(), nested));
                }
                Some(Field::Scalar(sf)) => {
                    index_fields.push(IndexField::scalar(sf.clone()));
                }
                _ => panic!(
                    "Unable to resolve field path '{}'",
                    grouped_paths.first().unwrap().join(".")
                ),
            }
        }

        index_fields
    }
}

pub fn walk_composite_paths(field_paths: FieldPaths) -> FieldPaths {
    field_paths
        .into_iter()
        .map(|path| {
            if let Some((_, rest)) = path.split_first() {
                rest.to_vec()
            } else {
                unreachable!()
            }
        })
        .collect_vec()
}
