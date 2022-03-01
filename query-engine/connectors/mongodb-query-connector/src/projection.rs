use crate::IntoBson;
use mongodb::bson::{Bson, Document};
use prisma_models::{FieldSelection, SelectedField};

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
            prisma_models::SelectedField::Scalar(sf) => {
                let mut parent_paths = parent_paths.clone();
                parent_paths.push(sf.db_name().to_owned());
                doc.insert(parent_paths.join("."), Bson::Int32(1));
            }

            prisma_models::SelectedField::Composite(cs) => {
                let mut parent_paths = parent_paths.clone();
                parent_paths.push(cs.field.db_name().to_owned());
                path_prefixed_selection(doc, parent_paths, cs.selections);
            }
        }
    }
}
