use super::expression::*;
use crate::IntoBson;

use mongodb::bson::{doc, Bson, Document};

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

impl IntoBson for UpdateExpression {
    fn into_bson(self) -> crate::Result<Bson> {
        match self {
            UpdateExpression::Set(set) => set.into_bson(),
            UpdateExpression::IfThenElse(if_then_else) => if_then_else.into_bson(),
            UpdateExpression::MergeDocument(merge_doc) => merge_doc.into_bson(),
            UpdateExpression::Generic(bson) => Ok(bson),
        }
    }
}
