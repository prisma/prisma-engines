use crate::{Field, Index, IndexField, IndexType};

use itertools::Itertools;

type FieldPath = Vec<String>;

#[derive(Debug)]
pub struct IndexBuilder {
    pub name: Option<String>,
    pub field_paths: Vec<FieldPath>,
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

    /// Takes in a `Vec<FieldPath>` and returns `Vec<IndexField>`.
    ///
    /// Consider this unique index: @@unique([a, b.c, b.d.e])
    /// The `field_paths` input of this function would be: [["a"], ["b", "c"], ["b", "d", "e"]]
    /// We're looking to produce this output:
    /// [
    ///   Scalar("a"),
    ///   Composite("b", [
    ///    Scalar("c"),
    ///    Composite("d", [
    ///      Scalar("e")
    ///    ])
    ///   ])
    /// ]
    ///
    /// Here are the steps we take:
    /// 1. Group the field paths by the first item of their paths -> [("a", [["a"]]), ("b", [["b", "c"], ["b", "d", "e"]])]
    /// 2. Iterate over each groups and find fields.
    ///   a. Is "a" a scalar field? yes. -> Append IndexField::Scalar("a")
    ///   b. Is "b" a composite field? yes. -> Append IndexField::Composite("b", [...])
    /// 3. When encountering a composite field, recursively call this function with:
    ///   a: The "consumed" composite field paths. eg: [["b", "c"], ["b", "d", "e"]] -> [["c"], ["d", "e"]]
    ///   b: The fields of the type of the composite field we're building
    /// 4. (in the recursion) Group the field paths by the first item of their paths -> [("c", [["c"]]), ("d", [["d", "e"]])]
    /// 5. Iterate over each groups and find fields
    ///   a. Is "c" a scalar field? yes. -> append `IndexField::Scalar("c")`
    ///   b. Is "d" a composite field? yes. -> Append `IndexField::Composite("d", [...])`
    /// 6. When encountering a composite field, recursively call this function with... (and so on...)
    /// 7. Return index fields
    fn build_index_fields(field_paths: Vec<FieldPath>, fields: &[Field]) -> Vec<IndexField> {
        let mut index_fields: Vec<IndexField> = vec![];

        for (field_name, grouped_paths) in &field_paths.into_iter().group_by(|path| path.first().cloned().unwrap()) {
            let grouped_paths = grouped_paths.collect_vec();
            let field = fields.iter().find(|f| f.name() == field_name);

            match field {
                Some(Field::Scalar(sf)) => {
                    index_fields.push(IndexField::scalar(sf.clone()));
                }
                Some(Field::Composite(cf)) => {
                    let walked_paths = consume_composite_paths(grouped_paths);
                    let nested = Self::build_index_fields(walked_paths, cf.typ.fields());

                    index_fields.push(IndexField::composite(cf.clone(), nested));
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

/// Consume the first item of each field paths. Used to recursively extract composite indexex.
/// eg: [["a", "b", "c"], ["a", "b", "d"]] -> [["b", "c"], ["b", "d"]]
pub fn consume_composite_paths(field_paths: Vec<FieldPath>) -> Vec<FieldPath> {
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
