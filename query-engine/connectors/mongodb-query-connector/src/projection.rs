use crate::IntoBson;
use bson::{Bson, Document};
use query_structure::{FieldSelection, SelectedField};

/// Used as projection document for Mongo queries.
impl IntoBson for FieldSelection {
    fn into_bson(self) -> crate::Result<Bson> {
        let mut doc = Document::new();
        path_prefixed_selection(&mut doc, vec![], self.into_inner());

        Ok(doc.into())
    }
}

fn path_prefixed_selection(doc: &mut Document, parent_paths: Vec<String>, selections: Vec<SelectedField>) {
    for field in selections {
        match field {
            query_structure::SelectedField::Scalar(sf) => {
                let mut parent_paths = parent_paths.clone();
                parent_paths.push(sf.db_name().to_owned());
                doc.insert(parent_paths.join("."), Bson::Int32(1));
            }

            query_structure::SelectedField::Composite(cs) => {
                let mut parent_paths = parent_paths.clone();
                parent_paths.push(cs.field.db_name().to_owned());
                path_prefixed_selection(doc, parent_paths, cs.selections);
            }

            query_structure::SelectedField::Relation(_) => unreachable!(),

            query_structure::SelectedField::Virtual(_) => {}
        }
    }
}
