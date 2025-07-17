use super::expression::*;
use crate::IntoBson;

use bson::{Bson, Document, doc};
use itertools::Itertools;

impl IntoBson for Set {
    fn into_bson(self) -> crate::Result<Bson> {
        let doc = doc! {
            "$set": { self.field_path.path(true): (*self.expression).into_bson()? }
        };

        Ok(Bson::from(doc))
    }
}

impl IntoBson for IfThenElse {
    fn into_bson(self) -> crate::Result<Bson> {
        let doc = doc! {
            "$cond": {
                "if": (*self.cond).into_bson()?,
                "then": (*self.then).into_bson()?,
                "else": (*self.els).into_bson()?
            }
        };

        Ok(Bson::from(doc))
    }
}

impl IntoBson for MergeDocument {
    fn into_bson(self) -> crate::Result<Bson> {
        let mut doc = Document::default();

        for (k, v) in self.inner {
            doc.insert(k, v.into_bson()?);
        }

        Ok(Bson::from(doc))
    }
}

impl IntoBson for MergeObjects {
    fn into_bson(self) -> crate::Result<Bson> {
        let input: Bson = if self.keys_to_unset.is_empty() {
            self.field_path().dollar_path(true).into()
        } else {
            let ands = self
                .keys_to_unset
                .iter()
                .map(|target| doc! { "$ne": ["$$elem.k", target] })
                .collect_vec();

            doc! {
              "$arrayToObject": {
                "$filter": {
                  "input": { "$objectToArray": self.field_path().dollar_path(true) },
                  "as": "elem",
                  "cond": { "$and": ands }
                }
              }
            }
            .into()
        };

        let doc = doc! {
            "$mergeObjects": [input, self.document.into_bson()?]
        };

        Ok(Bson::from(doc))
    }
}

impl IntoBson for UpdateExpression {
    fn into_bson(self) -> crate::Result<Bson> {
        match self {
            UpdateExpression::Set(set) => set.into_bson(),
            UpdateExpression::IfThenElse(if_then_else) => if_then_else.into_bson(),
            UpdateExpression::MergeDocument(merge_doc) => merge_doc.into_bson(),
            UpdateExpression::Generic(bson) => Ok(bson),
            UpdateExpression::MergeObjects(merge_objects) => merge_objects.into_bson(),
        }
    }
}
